use anyhow::{anyhow, Error};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use futures::future::join_all;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::models::dto::{Coin, CoinBalanceResponse, Transaction, TransactionResponse};
use crate::{
    database,
    models::{MarketCap, SwapTransaction, TokenHolderError, TokenTerminalData},
};
use headless_chrome::{Browser, LaunchOptionsBuilder};

const FULLNODE_API: &str = "https://api.mainnet.aptoslabs.com/v1";
pub const USDT: &str =
    "0xf22bede237a07e121b56d91a491eb7bcdfd1f5907926a9e58338f964a01b17fa::asset::USDT";
pub const USDC: &str =
    "0xf22bede237a07e121b56d91a491eb7bcdfd1f5907926a9e58338f964a01b17fa::asset::USDC";
const DECIMALS_USD: u8 = 6;

#[derive(Clone)]
pub struct External {
    pub client: Client,
}

impl Default for External {
    fn default() -> Self {
        Self::new()
    }
}

impl External {
    pub fn new() -> Self {
        External {
            client: Client::new(),
        }
    }

    /// ~10s and takes ~1600 APIs
    /// Should save this value to DB and only call this once a day to update it.
    pub async fn get_total_value_locked(&self, address: &str) -> Result<f64, reqwest::Error> {
        let res: Value = self
            .client
            .get(format!("{FULLNODE_API}/accounts/{address}/resources"))
            .send()
            .await?
            .json()
            .await?;

        let mut reserves: HashMap<String, u64> = HashMap::new();

        if let Some(array) = res.as_array() {
            for obj in array {
                if let Some(obj_type) = obj.get("type").and_then(Value::as_str) {
                    if obj_type.contains("swap::TokenPairReserve") {
                        if let Some(data) = obj.get("data").and_then(Value::as_object) {
                            if let (Some(reserve_x), Some(reserve_y)) =
                                (data.get("reserve_x"), data.get("reserve_y"))
                            {
                                if let (Some(reserve_x_str), Some(reserve_y_str)) =
                                    (reserve_x.as_str(), reserve_y.as_str())
                                {
                                    let reserve_x_value = reserve_x_str.parse::<u64>().unwrap_or(0);
                                    let reserve_y_value = reserve_y_str.parse::<u64>().unwrap_or(0);

                                    let (token_x, token_y) =
                                        Self::get_token_names_from_type(obj_type);
                                    *reserves.entry(token_x.to_string()).or_insert(0) +=
                                        reserve_x_value;
                                    *reserves.entry(token_y.to_string()).or_insert(0) +=
                                        reserve_y_value;
                                }
                            }
                        }
                    }
                }
            }
        }

        let total_value_locked = self.calculate_total_value_locked(&reserves).await;
        println!("Total Value Locked: ${:.2}", total_value_locked);

        Ok(total_value_locked)
    }

    async fn calculate_total_value_locked(&self, reserves: &HashMap<String, u64>) -> f64 {
        let mut total_value_locked = 0.0;
        let mut tasks = Vec::new();

        for (token, &reserve) in reserves {
            let token_clone = token.to_string();
            let reserve_clone = reserve;
            let client = self.client.clone();

            let task = tokio::task::spawn(async move {
                if let Some((price, decimals)) =
                    External::get_price_and_decimals(client, &token_clone).await
                {
                    (price * reserve_clone as f64) / 10f64.powi(decimals as i32)
                } else {
                    0.0
                }
            });
            tasks.push(task);
        }

        for task in tasks {
            total_value_locked += task.await.unwrap_or(0.0);
        }

        total_value_locked
    }

    pub async fn get_price_and_decimals(client: Client, token: &str) -> Option<(f64, u8)> {
        if token == USDT || token == USDC {
            return Some((1.0, DECIMALS_USD));
        }

        let decimals_future = External::get_decimals(&client, token);
        let usdc_balance_future = External::get_balances(&client, token, USDC);
        let usdt_balance_future = External::get_balances(&client, token, USDT);

        let decimals = decimals_future.await?;

        let (usdc_result, usdt_result) = tokio::join!(usdc_balance_future, usdt_balance_future);

        if let Some((balance_x, balance_y)) = usdc_result {
            let price = (balance_y as f64) / (balance_x as f64)
                * 10f64.powi(decimals as i32 - DECIMALS_USD as i32);
            return Some((price, decimals));
        }

        if let Some((balance_x, balance_y)) = usdt_result {
            let price = (balance_y as f64) / (balance_x as f64)
                * 10f64.powi(decimals as i32 - DECIMALS_USD as i32);
            return Some((price, decimals));
        }

        None
    }

    async fn get_decimals(client: &Client, token: &str) -> Option<u8> {
        let graphql_query = format!(
            r#"
            query MyQuery {{
                coin_infos(where: {{coin_type: {{_eq: "{}"}}}}) {{
                    decimals
                }}
            }}"#,
            token
        );

        let response: Value = client
            .post(format!("{FULLNODE_API}/graphql"))
            .json(&serde_json::json!({ "query": graphql_query }))
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()?;

        response["data"]["coin_infos"]
            .as_array()?
            .first()?
            .get("decimals")?
            .as_u64()
            .map(|d| d as u8)
    }

    async fn get_balances(client: &Client, token: &str, stablecoin: &str) -> Option<(i64, i64)> {
        async fn fetch_balances(client: &Client, token1: &str, token2: &str) -> Option<(i64, i64)> {
            let response: Value = client
            .get(format!(
                "{FULLNODE_API}/accounts/0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa/resource/0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa::swap::TokenPairMetadata<{},{}>",
                token1, token2
            ))
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()?;

            let balance_x: i64 = response["data"]["balance_x"]["value"]
                .as_str()?
                .parse()
                .ok()?;
            let balance_y: i64 = response["data"]["balance_y"]["value"]
                .as_str()?
                .parse()
                .ok()?;

            Some((balance_x, balance_y))
        }

        // Try with original order
        if let Some(balances) = fetch_balances(client, token, stablecoin).await {
            return Some(balances);
        }

        // If original order fails, try with swapped order
        if let Some((balance_y, balance_x)) = fetch_balances(client, stablecoin, token).await {
            return Some((balance_x, balance_y)); // Swap back the order of balances
        }

        None // If both attempts fail, return None
    }

    /// Use headless chrome to extract the data.
    /// Note that it needs to wait for a few seconds (3) to load the data.
    /// Consider increasing it if sometimes the data couldn't be fetched.
    pub async fn get_data_from_tokenterminal(
        &self,
        project: &str,
    ) -> Result<TokenTerminalData, Error> {
        // Initialize the browser with headless mode
        let browser = Browser::new(
            LaunchOptionsBuilder::default()
                .headless(true)
                .sandbox(false)
                .build()?,
        )?;

        // Create a new tab and navigate to the project page
        let tab = browser.new_tab()?;
        tab.navigate_to(&format!(
            "https://tokenterminal.com/terminal/projects/{project}"
        ))?;

        // Wait for the page to load (consider using a more robust waiting mechanism)
        tokio::time::sleep(std::time::Duration::from_secs(4)).await;

        // Get the page content
        let html = tab.get_content()?;
        let document = Html::parse_document(&html);

        // Scrape ATH/ATL data
        let (ath, ath_last, atl, atl_last) = self.scrape_ath_atl(&document)?;

        // Scrape financial data
        let mut data = self.scrape_financials(&document)?;

        // Add ATH/ATL data to the TokenTerminalData struct
        data.ath = ath;
        data.ath_last = ath_last;
        data.atl = atl;
        data.atl_last = atl_last;

        Ok(data)
    }

    fn scrape_ath_atl(&self, document: &Html) -> Result<(String, String, String, String), Error> {
        let span_selector =
            Selector::parse("span").map_err(|e| anyhow!("Failed to parse selector: {}", e))?;
        let mut ath = String::new();
        let mut ath_last = String::new();
        let mut atl = String::new();
        let mut atl_last = String::new();
        let mut spans = document.select(&span_selector).peekable();

        while let Some(span) = spans.next() {
            let text = span.text().collect::<String>();
            match text.as_str() {
                "ATH" => {
                    ath = spans.next().map(|s| s.text().collect()).unwrap_or_default();
                    ath_last = spans.next().map(|s| s.text().collect()).unwrap_or_default();
                }
                "ATL" => {
                    atl = spans.next().map(|s| s.text().collect()).unwrap_or_default();
                    atl_last = spans.next().map(|s| s.text().collect()).unwrap_or_default();
                }
                _ => continue,
            }
        }

        Ok((ath, ath_last, atl, atl_last))
    }

    fn scrape_financials(&self, document: &Html) -> Result<TokenTerminalData, Error> {
        let li_selector =
            Selector::parse("li").map_err(|e| anyhow!("Failed to parse li selector: {}", e))?;
        let div_selector =
            Selector::parse("div").map_err(|e| anyhow!("Failed to parse div selector: {}", e))?;
        let mut data = TokenTerminalData::default();

        for li in document.select(&li_selector) {
            let mut divs = li.select(&div_selector);
            if let (Some(label_div), Some(value_div)) = (divs.next(), divs.next()) {
                let label = label_div.text().collect::<String>();
                let value = value_div
                    .text()
                    .collect::<Vec<_>>()
                    .first()
                    .cloned()
                    .unwrap_or_default()
                    .to_owned();

                match label.as_str() {
                    l if l.contains("Revenue (30d)") => data.revenue_30d = value,
                    l if l.contains("Revenue (annualized)") => data.revenue_annualized = value,
                    l if l.contains("Expenses (30d)") => data.expenses_30d = value,
                    l if l.contains("Earnings (30d)") => data.earnings_30d = value,
                    l if l.contains("Fees (30d)") => data.fees_30d = value,
                    l if l.contains("Fees (annualized)") => data.fees_annualized = value,
                    l if l.contains("Token incentives (30d)") => data.token_incentives_30d = value,
                    l if l.contains("Active users (monthly)") => data.monthly_active_users = value,
                    l if l.contains("Average fees per user (AFPU)") => data.afpu = value,
                    l if l.contains("Average revenue per user (ARPU)") => data.arpu = value,
                    l if l.contains("Token trading volume (30d)") => {
                        data.token_trading_volume_30d = value
                    }
                    _ => {}
                }
            }
        }

        Ok(data)
    }

    /// Get 25 latest transactions impacting PancakeSwap
    pub async fn get_swap_transactions(
        &self,
        account_address: &str,
        entry_function_id_str: &str,
    ) -> Result<Vec<SwapTransaction>, reqwest::Error> {
        // GraphQL query with dynamic parameters
        let graphql_query = format!(
            r#"
        query AccountTransactionsData {{
            account_transactions(
                limit: 25
                where: {{
                    account_address: {{_eq: "{}"}}, 
                    user_transaction: {{entry_function_id_str: {{_eq: "{}"}}}}
                }}
                order_by: {{transaction_version: desc}}
            ) {{
                transaction_version
                user_transaction {{
                    sender
                }}
                coin_activities {{
                    activity_type
                    amount
                    coin_type
                    coin_info {{
                        decimals
                    }}
                }}
            }}
        }}"#,
            account_address, entry_function_id_str
        );

        // Sending the GraphQL query to the server
        let response: Value = self
            .client
            .post(format!("{}/graphql", FULLNODE_API))
            .json(&serde_json::json!({ "query": graphql_query }))
            .send()
            .await?
            .json()
            .await?;

        let mut transactions = Vec::new();

        // Parsing the response and creating SwapTransaction objects
        if let Some(array) = response["data"]["account_transactions"].as_array() {
            for transaction in array {
                let version = transaction["transaction_version"].as_i64().unwrap_or(0);
                let sender = transaction["user_transaction"]["sender"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                let mut token_sold = String::new();
                let mut token_sold_amount = 0.0;
                let mut token_bought = String::new();
                let mut token_bought_amount = 0.0;

                if let Some(activities) = transaction["coin_activities"].as_array() {
                    for activity in activities.iter().skip(1) {
                        let activity_type = activity["activity_type"].as_str().unwrap_or("");
                        let amount = activity["amount"].as_f64().unwrap_or(0.0);
                        let coin_type = activity["coin_type"].as_str().unwrap_or("").to_string();
                        let decimals =
                            activity["coin_info"]["decimals"].as_u64().unwrap_or(0) as u32;

                        let adjusted_amount = amount / 10f64.powi(decimals as i32);

                        match activity_type {
                            "0x1::coin::WithdrawEvent" => {
                                token_sold = coin_type;
                                token_sold_amount = adjusted_amount;
                            }
                            "0x1::coin::DepositEvent" => {
                                token_bought = coin_type;
                                token_bought_amount = adjusted_amount;
                            }
                            _ => {}
                        }
                    }
                }

                transactions.push(SwapTransaction {
                    version,
                    sender,
                    token_sold,
                    token_sold_amount,
                    token_bought,
                    token_bought_amount,
                });
            }
        }

        Ok(transactions)
    }
    pub async fn get_token_supply(&self, address: &str, token: &str) -> Result<f64, Error> {
        let url =
            format!("{FULLNODE_API}/accounts/{address}/resource/0x1::coin::CoinInfo<{token}>");

        let response: Value = self.client.get(&url).send().await?.json().await?;

        if let Some(data) = response["data"].as_object() {
            if let Some(decimals) = data["decimals"].as_u64() {
                if let Some(supply) =
                    data["supply"]["vec"][0]["integer"]["vec"][0]["value"].as_str()
                {
                    let supply_value: f64 = supply.parse()?;
                    let adjusted_supply = supply_value / 10f64.powi(decimals as i32);
                    return Ok(adjusted_supply);
                }
            }
        }

        Err(anyhow!("Failed to get token supply"))
    }
    pub async fn calculate_market_cap(
        &self,
        db: &database::PostgreDatabase,
        address: &str,
        token: &str,
        token_address: &str,
    ) -> Result<MarketCap, Error> {
        let client = Client::new();

        // Get the token price
        let price = match Self::get_price_and_decimals(client.clone(), token).await {
            Some((price, _)) => price,
            None => return Err(anyhow!("Failed to get price and decimals")),
        };

        // Get the max supply from the database
        let project = db.get_project_by_address(address).await?.unwrap();

        let circulating_supply = self.get_token_supply(token_address, token).await?;

        // Calculate fully diluted and normal market caps
        let fully_diluted = match project.get_int("token_max_supply") {
            Some(max_supply) => price * (max_supply as f64),
            None => 0.0, // or some other default value or handling logic
        };
        let normal = price * circulating_supply;

        Ok(MarketCap {
            fully_diluted,
            normal,
        })
    }

    // ~80 API calls and ~20s
    pub async fn get_number_of_token_holders(&self, token: &str) -> Result<u64, TokenHolderError> {
        let mut left = 1u64;
        let mut right = 1_000_000_000u64;

        while left <= right {
            //println!("{} - {}", left, right);
            let segment = (right - left + 1) / 10;
            if segment == 0 {
                break;
            }

            let mut tasks = Vec::new();
            for i in 0..10 {
                let offset = left + i * segment;
                let token = token.to_string();
                let client = self.client.clone();
                tasks.push(tokio::spawn(async move {
                    Self::query_coin_balances(&client, &token, offset).await
                }));
            }

            let results = futures::future::join_all(tasks).await;

            let mut found = false;
            for (i, task_result) in results.into_iter().enumerate() {
                match task_result {
                    Ok(Ok(count)) if count > 0 && count < 100 => {
                        let offset = left + i as u64 * segment;
                        return Ok(offset + count);
                    }
                    Ok(Ok(0)) => {
                        right = left + i as u64 * segment - 1;
                        left = if i > 0 {
                            left + (i as u64 - 1) * segment
                        } else {
                            left
                        };
                        found = true;
                        break;
                    }
                    Ok(Err(e)) => return Err(e),
                    Err(e) => return Err(TokenHolderError::ApiError(e.to_string())),
                    _ => continue,
                }
            }

            if !found {
                left = right - segment + 1;
            }
        }

        Ok(left)
    }

    async fn query_coin_balances(
        client: &Client,
        token: &str,
        offset: u64,
    ) -> Result<u64, TokenHolderError> {
        let query = format!(
            r#"
            query MyQuery {{
                current_coin_balances(
                    offset: {}
                    limit: 100
                    where: {{coin_type: {{_eq: "{}"}}, amount: {{_gt: "0"}}}}
                ) {{
                    amount
                }}
            }}
            "#,
            offset, token
        );

        let response: Value = client
            .post(format!("{FULLNODE_API}/graphql"))
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await?
            .json()
            .await?;

        let count = response["data"]["current_coin_balances"]
            .as_array()
            .map(|arr| arr.len())
            .unwrap_or(0);

        Ok(count as u64)
    }

    pub async fn calculate_trading_volume(
        &self,
        address: &str,
        entry_function_id: &str,
    ) -> Result<f64, Error> {
        let client = Arc::new(self.client.clone());
        let coin_volumes: Arc<Mutex<HashMap<String, u64>>> = Arc::new(Mutex::new(HashMap::new()));
        let mut offset = 0;
        let mut found_old_activity = false;
        let now = Utc::now();
        let seven_days_ago = now - Duration::days(7);

        while !found_old_activity {
            let mut tasks = Vec::new();

            for _ in 0..250 {
                let client = Arc::clone(&client);
                let coin_volumes = Arc::clone(&coin_volumes);
                let address = address.to_string();
                let entry_function_id = entry_function_id.to_string();
                let current_offset = offset;

                let task = tokio::spawn(async move {
                    let query = format!(
                        r#"
                        query AccountTransactionsData {{
                            account_transactions(
                                offset: {}
                                limit: 100
                                where: {{account_address: {{_eq: "{}"}}, user_transaction: {{entry_function_id_str: {{_eq: "{}"}}}}}}
                                order_by: {{transaction_version: desc}}
                            ) {{
                                coin_activities {{
                                    amount
                                    coin_info {{
                                        coin_type
                                    }}
                                    transaction_timestamp
                                }}
                            }}
                        }}
                        "#,
                        current_offset, address, entry_function_id
                    );

                    let response: Value = client
                        .post(format!("{}/graphql", FULLNODE_API))
                        .json(&serde_json::json!({ "query": query }))
                        .send()
                        .await?
                        .json()
                        .await?;

                    let mut local_found_old_activity = false;

                    if let Some(transactions) = response["data"]["account_transactions"].as_array()
                    {
                        for transaction in transactions {
                            if let Some(activities) = transaction["coin_activities"].as_array() {
                                for activity in activities {
                                    //println!("{}", activity);
                                    if let Some(raw_timestamp) =
                                        activity["transaction_timestamp"].as_str()
                                    {
                                        // Parse the timestamp using NaiveDateTime
                                        match NaiveDateTime::parse_from_str(
                                            raw_timestamp,
                                            "%Y-%m-%dT%H:%M:%S",
                                        ) {
                                            Ok(naive_dt) => {
                                                let utc_time =
                                                    DateTime::<Utc>::from_naive_utc_and_offset(
                                                        naive_dt, Utc,
                                                    );

                                                if utc_time < seven_days_ago {
                                                    local_found_old_activity = true;
                                                    break;
                                                }

                                                let amount =
                                                    activity["amount"].as_u64().unwrap_or(0);
                                                let coin_type = activity["coin_info"]["coin_type"]
                                                    .as_str()
                                                    .unwrap_or("");

                                                let mut volumes = coin_volumes.lock().await;
                                                *volumes
                                                    .entry(coin_type.to_string())
                                                    .or_insert(0) += amount;
                                            }
                                            Err(e) => println!("Failed to parse timestamp: {}", e),
                                        }
                                    } else {
                                        println!("No timestamp found in activity");
                                    }
                                }
                            }
                            if local_found_old_activity {
                                break;
                            }
                        }
                    }

                    Ok::<bool, Error>(local_found_old_activity)
                });

                tasks.push(task);
                offset += 100;
            }

            let results = join_all(tasks).await;
            for result in results {
                match result {
                    Ok(Ok(local_found_old_activity)) => {
                        if local_found_old_activity {
                            found_old_activity = true;
                            break;
                        }
                    }
                    Ok(Err(e)) => return Err(e),
                    Err(e) => return Err(Error::from(e)),
                }
            }
        }

        let coin_volumes = Arc::try_unwrap(coin_volumes)
            .expect("Unable to unwrap Arc")
            .into_inner();

        let mut total_volume_usd = 0.0;
        let mut price_tasks = Vec::new();

        for (coin_type, volume) in coin_volumes.iter() {
            let client = self.client.clone();
            let coin_type = coin_type.clone();
            let volume = *volume;

            let task = tokio::spawn(async move {
                if let Some((price, decimals)) =
                    Self::get_price_and_decimals(client, &coin_type).await
                {
                    let volume_usd = price * (volume as f64) / 10f64.powi(decimals as i32);
                    Ok(volume_usd)
                } else {
                    Err(format!(
                        "Failed to get price and decimals of {}",
                        &coin_type
                    ))
                }
            });

            price_tasks.push(task);
        }

        let results = join_all(price_tasks).await;
        for result in results {
            match result {
                Ok(Ok(volume_usd)) => total_volume_usd += volume_usd,
                Ok(Err(e)) => eprintln!("Error calculating volume: {}", e),
                Err(e) => eprintln!("Task error: {}", e),
            }
        }

        Ok(total_volume_usd)
    }

    pub async fn get_daily_active_users(&self, address: &str) -> Result<usize, Error> {
        let client = Arc::new(self.client.clone());
        let mut offset = 0;
        let mut active_users = HashSet::new();
        let today = Utc::now().date_naive();
        let mut found_old_transaction = false;

        while !found_old_transaction {
            let mut tasks = Vec::new();

            for _ in 0..50 {
                let client = Arc::clone(&client);
                let address = address.to_string();
                let current_offset = offset;

                let task = tokio::spawn(async move {
                    let query = format!(
                        r#"
                        query AccountTransactionsData {{
                            account_transactions(
                                offset: {}
                                limit: 100
                                where: {{account_address: {{_eq: "{}"}}}},
                                order_by: {{transaction_version: desc}}
                            ) {{
                                user_transaction {{
                                    sender
                                    timestamp
                                }}
                            }}
                        }}
                        "#,
                        current_offset, address
                    );

                    let response: Value = client
                        .post(format!("{}/graphql", FULLNODE_API))
                        .json(&serde_json::json!({ "query": query }))
                        .send()
                        .await?
                        .json()
                        .await?;

                    let mut daily_users = HashSet::new();
                    let mut batch_found_old_transaction = false;

                    if let Some(transactions) = response["data"]["account_transactions"].as_array()
                    {
                        for transaction in transactions {
                            if let Some(user_transaction) =
                                transaction["user_transaction"].as_object()
                            {
                                if let (Some(sender), Some(timestamp)) = (
                                    user_transaction["sender"].as_str(),
                                    user_transaction["timestamp"].as_str(),
                                ) {
                                    if let Ok(transaction_time) = NaiveDateTime::parse_from_str(
                                        timestamp,
                                        "%Y-%m-%dT%H:%M:%S%.f",
                                    ) {
                                        let transaction_date = transaction_time.date();
                                        if transaction_date == today {
                                            daily_users.insert(sender.to_string());
                                        } else {
                                            batch_found_old_transaction = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Ok::<(HashSet<String>, bool), Error>((daily_users, batch_found_old_transaction))
                });

                tasks.push(task);
                offset += 100;
            }

            let results = join_all(tasks).await;

            for result in results {
                match result {
                    Ok(Ok((users, batch_old_transaction))) => {
                        active_users.extend(users);
                        if batch_old_transaction {
                            found_old_transaction = true;
                        }
                    }
                    Ok(Err(e)) => eprintln!("Error in task: {}", e),
                    Err(e) => eprintln!("Task join error: {}", e),
                }
            }

            println!("Processed {} transactions", offset);
        }

        println!("Total API calls made: {}", offset / 100);
        println!("Found all transactions for today");

        Ok(active_users.len())
    }

    pub async fn get_weekly_active_users(&self, address: &str) -> Result<usize, Error> {
        let client = Arc::new(self.client.clone());
        let mut offset = 0;
        let mut active_users = HashSet::new();
        let now = Utc::now();
        let seven_days_ago = (now - Duration::days(7)).date_naive();
        let mut found_old_transaction = false;

        while !found_old_transaction {
            let mut tasks = Vec::new();

            for _ in 0..250 {
                let client = Arc::clone(&client);
                let address = address.to_string();
                let current_offset = offset;

                let task = tokio::spawn(async move {
                    let query = format!(
                        r#"
                        query AccountTransactionsData {{
                            account_transactions(
                                offset: {}
                                limit: 100
                                where: {{account_address: {{_eq: "{}"}}}},
                                order_by: {{transaction_version: desc}}
                            ) {{
                                user_transaction {{
                                    sender
                                    timestamp
                                }}
                            }}
                        }}
                        "#,
                        current_offset, address
                    );

                    let response: Value = client
                        .post(format!("{}/graphql", FULLNODE_API))
                        .json(&serde_json::json!({ "query": query }))
                        .send()
                        .await?
                        .json()
                        .await?;

                    let mut weekly_users = HashSet::new();
                    let mut batch_found_old_transaction = false;

                    if let Some(transactions) = response["data"]["account_transactions"].as_array()
                    {
                        for transaction in transactions {
                            if let Some(user_transaction) =
                                transaction["user_transaction"].as_object()
                            {
                                if let (Some(sender), Some(timestamp)) = (
                                    user_transaction["sender"].as_str(),
                                    user_transaction["timestamp"].as_str(),
                                ) {
                                    if let Ok(transaction_time) = NaiveDateTime::parse_from_str(
                                        timestamp,
                                        "%Y-%m-%dT%H:%M:%S%.f",
                                    ) {
                                        let transaction_date = transaction_time.date();
                                        if transaction_date >= seven_days_ago {
                                            weekly_users.insert(sender.to_string());
                                        } else {
                                            batch_found_old_transaction = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Ok::<(HashSet<String>, bool), Error>((
                        weekly_users,
                        batch_found_old_transaction,
                    ))
                });

                tasks.push(task);
                offset += 100;
            }

            let results = join_all(tasks).await;

            for result in results {
                match result {
                    Ok(Ok((users, batch_old_transaction))) => {
                        active_users.extend(users);
                        if batch_old_transaction {
                            found_old_transaction = true;
                        }
                    }
                    Ok(Err(e)) => eprintln!("Error in task: {}", e),
                    Err(e) => eprintln!("Task join error: {}", e),
                }
            }

            println!("Processed {} transactions", offset);

            // Break if we've processed a very large number of transactions to prevent infinite loops
            if offset >= 500_000 {
                println!("Reached 500,000 transactions processed. Stopping to prevent excessive API calls.");
                break;
            }
        }

        println!("Total API calls made: {}", offset / 100);
        if found_old_transaction {
            println!("Found all transactions for the last 7 days");
        } else {
            println!("Warning: Stopped due to large number of transactions. May not have all 7 days of data.");
        }

        Ok(active_users.len())
    }

    async fn graphql(client: &Client, graphql_query: &String) -> Option<Value> {
        client
            .post("https://indexer.mainnet.aptoslabs.com/v1/graphql")
            .json(&serde_json::json!({ "query": graphql_query }))
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()?
    }

    // from "ABC<DEF>" -> "DEF", additionaly remove space
    fn get_generic_type(input: &str) -> String {
        let input = input.replace(" ", "");
        if let Some(pos) = input.find('<') {
            input[pos + 1..input.len() - 1].to_string()
        } else {
            input.to_string()
        }
    }

    // pair has syntax of "tokenA,tokenB"
    fn get_token_names_from_pair(input: &str) -> (String, String) {
        let mut num_open_bracket = 0;
        let mut comma_position = 0;

        for (i, c) in input.chars().enumerate() {
            match c {
                '<' => num_open_bracket += 1,
                '>' => num_open_bracket -= 1,
                ',' if num_open_bracket == 0 => {
                    comma_position = i;
                    break;
                }
                _ => {}
            }
        }

        (
            input[0..comma_position].to_owned(),
            input[comma_position + 1..].to_owned(),
        )
    }

    // get full tokenA, tokenB from "address::name::type<tokenA, tokenB>"
    fn get_token_names_from_type(input: &str) -> (String, String) {
        let input = Self::get_generic_type(input);
        Self::get_token_names_from_pair(input.as_str())
    }

    pub async fn get_fee_within_n_days_pancake(&self, day: i64) -> Result<f64, reqwest::Error> {
        let now = Utc::now();
        let n_days_ago = (now - Duration::days(day)).date_naive();
        let mut offset = 0;

        let mut tasks = Vec::new();

        // this 250 cap is not enough, should save this to db
        for _ in 0..250 {
            let client_clone = self.client.clone();
            let current_offset = offset;
            let task = tokio::task::spawn(async move {
                let graphql_query = format!(
                    r#"
                    query MyQuery {{
                        events(
                            where: {{indexed_type: {{_like: "0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa::swap::SwapEvent%"}}}}
                            order_by: {{transaction_version: desc}}
                            offset: {current_offset}
                        ) {{
                            data
                            indexed_type
                            transaction_version
                        }}
                    }}"#
                );

                if let Some(swap_events) = Self::graphql(&client_clone, &graphql_query).await {
                    if let Some(array) = swap_events["data"]["events"].as_array() {
                        if array.is_empty() {
                            return (Vec::new(), None);
                        }
                        // query transaction with this transaction_version to check timestamp
                        let transaction_version = array.last().unwrap()["transaction_version"]
                            .as_number()
                            .unwrap();
                        let graphql_query = format!(
                            r#"
                            query MyQuery {{
                                account_transactions(
                                    where: {{transaction_version: {{_eq: "{transaction_version}"}}}}
                                    limit: 1
                                ) {{
                                    user_transaction {{
                                        timestamp
                                    }}
                                }}
                            }}
                        "#
                        );
                        let query_returned =
                            Self::graphql(&client_clone, &graphql_query).await.unwrap();
                        let transactions = query_returned["data"]["account_transactions"]
                            .as_array()
                            .unwrap();
                        let transaction = &transactions[0];
                        let timestamp = &transaction["user_transaction"]["timestamp"]
                            .as_str()
                            .unwrap();
                        let transaction_time =
                            NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%dT%H:%M:%S%.f")
                                .unwrap();
                        let transaction_date = transaction_time.date();
                        if transaction_date <= n_days_ago {
                            return (Vec::new(), Some(transaction_date));
                        };
                        let mut local_coin_swaps = Vec::new();
                        for obj in array {
                            let amount_x_in = &obj["data"]["amount_x_in"]
                                .as_str()
                                .unwrap()
                                .parse::<u64>()
                                .unwrap();
                            let amount_y_in = &obj["data"]["amount_y_in"]
                                .as_str()
                                .unwrap()
                                .parse::<u64>()
                                .unwrap();
                            let indexed_type = obj["indexed_type"].as_str().unwrap();
                            let (token_x, token_y) = Self::get_token_names_from_type(indexed_type);
                            if *amount_x_in > 0 {
                                local_coin_swaps.push((token_x, *amount_x_in));
                            };
                            if *amount_y_in > 0 {
                                local_coin_swaps.push((token_y, *amount_y_in));
                            };
                        }
                        return (local_coin_swaps, Some(transaction_date));
                    }
                }
                (Vec::new(), None)
            });
            tasks.push(task);
            offset += 100;
        }

        let mut total_coin_swapped: HashMap<String, u64> = HashMap::new();
        let mut optional_earliest_day_found = None;
        for task in tasks {
            let (local_total_coin_swapped, optional_day) = task.await.unwrap_or((Vec::new(), None));

            if optional_day.is_some() {
                optional_earliest_day_found = optional_day;
            };

            for (token, amount) in local_total_coin_swapped {
                *total_coin_swapped.entry(token.to_string()).or_insert(0) += amount;
            }
        }

        if let Some(earliest_day) = optional_earliest_day_found {
            println!("now: {:?}", now.date_naive());
            println!("earliest_day: {:?}", earliest_day);
        }

        Ok(Self::calculate_fee(self, total_coin_swapped, 25, 10000).await)
    }

    async fn calculate_fee(
        &self,
        total_coin_swapped: HashMap<String, u64>,
        numerator: u64,
        denomerator: u64,
    ) -> f64 {
        let mut tasks = Vec::new();
        let mut total_fee: f64 = 0f64;
        // fee is (numerator) / (denomerator)
        // value after fee is (denomerator - numerator) / (denomerator)
        // value after fee -> fee is (value after fee / (denomerator - numerator)) * (numerator) = (value after fee) / ((denomerator - numerator) / numerator)
        let divisor = ((denomerator - numerator) as f64) / (numerator as f64);

        for (token, amount) in &total_coin_swapped {
            let token_clone = token.to_string();
            let amount_clone = *amount;
            let divisor_clone = divisor;
            let client = self.client.clone();

            let task = tokio::task::spawn(async move {
                if let Some((price, decimals)) =
                    Self::get_price_and_decimals(client, &token_clone).await
                {
                    let fee_in_token = (amount_clone as f64) / divisor_clone;
                    (price * fee_in_token) / 10f64.powi(decimals as i32)
                } else {
                    0.0
                }
            });
            tasks.push(task);
        }

        for task in tasks {
            total_fee += task.await.unwrap_or(0.0);
        }

        total_fee
    }

    pub async fn fetch_coin_balances(&self, address: &str) -> Result<Vec<Coin>, reqwest::Error> {
        let query = format!(
            r#"
        query {{
          current_fungible_asset_balances(
            where: {{owner_address: {{_eq: "{}"}}}}
          ) {{
            amount_v1
            asset_type_v1
            metadata {{
              decimals
              name
              symbol
            }}
          }}
        }}
        "#,
            address
        );

        let res: CoinBalanceResponse = self
            .client
            .post(format!("{FULLNODE_API}/graphql"))
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await?
            .json()
            .await?;

        Ok(res
            .data
            .current_fungible_asset_balances
            .into_iter()
            .map(|balance| {
                let amount = (balance.amount_v1 as f64) / 10f64.powi(balance.metadata.decimals);
                Coin {
                    asset_type: balance.asset_type_v1,
                    name: balance.metadata.name,
                    symbol: balance.metadata.symbol,
                    amount,
                }
            })
            .collect())
    }

    pub async fn fetch_transactions(
        &self,
        address: &str,
    ) -> Result<Vec<Transaction>, reqwest::Error> {
        let query = format!(
            r#"
        query {{
          account_transactions(
            where: {{account_address: {{_eq: "{}"}}}}
            order_by: {{transaction_version: desc}}
            limit: 25
          ) {{
            transaction_version
            user_transaction {{
              entry_function_id_str
              timestamp
              sender
            }}
            coin_activities {{
              amount
              coin_type
              activity_type
            }}
          }}
        }}
        "#,
            address
        );

        let res: TransactionResponse = self
            .client
            .post(format!("{FULLNODE_API}/graphql"))
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await?
            .json()
            .await?;

        Ok(res
            .data
            .account_transactions
            .into_iter()
            .map(|tx| {
                let gas_fee = tx
                    .coin_activities
                    .iter()
                    .find(|activity| activity.activity_type == "0x1::aptos_coin::GasFeeEvent")
                    .map(|activity| activity.amount)
                    .unwrap_or(0);

                let amount = tx
                    .coin_activities
                    .iter()
                    .find(|activity| {
                        activity.coin_type == "0x1::aptos_coin::AptosCoin"
                            && activity.activity_type == "0x1::coin::WithdrawEvent"
                    })
                    .map(|activity| activity.amount)
                    .unwrap_or(0);

                let receiver = tx
                    .user_transaction
                    .entry_function_id_str
                    .split("::")
                    .next()
                    .unwrap_or("")
                    .to_string();

                Transaction {
                    version: tx.transaction_version,
                    timestamp: tx.user_transaction.timestamp,
                    sender: tx.user_transaction.sender,
                    receiver,
                    function: tx.user_transaction.entry_function_id_str,
                    amount,
                    gas_amount: gas_fee,
                }
            })
            .collect())
    }
}

#[tokio::test]
async fn test_get_data_from_tokenterminal() {
    let external = External::new();

    let result = external
        .get_data_from_tokenterminal("pancakeswap")
        .await
        .unwrap();

    assert_eq!(result.ath, "$42.46");
    assert_eq!(result.ath_last, "3.4y ago");
    assert_eq!(result.atl, "$0.2234");
    assert_eq!(result.atl_last, "3.9y ago");
    assert_eq!(result.revenue_30d, "$4.32m");
    assert_eq!(result.revenue_annualized, "$52.56m");
    assert_eq!(result.expenses_30d, "$2.08m");
    assert_eq!(result.earnings_30d, "$2.24m");
    assert_eq!(result.fees_30d, "$13.30m");
    assert_eq!(result.fees_annualized, "$161.79m");
    assert_eq!(result.token_incentives_30d, "$2.08m");
    assert_eq!(result.monthly_active_users, "1.98m");
    assert_eq!(result.afpu, "$1.62");
    assert_eq!(result.arpu, "$0.5293");
    assert_eq!(result.token_trading_volume_30d, "$1.29b");
}

#[tokio::test]
async fn test_get_swap_transactions() {
    let external = External::new();

    match external.get_swap_transactions("0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa", "0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa::router::swap_exact_input").await {
        Ok(transactions) => {
            for transaction in transactions {
                println!(
                    "Version: {}, Sender: {}, Token Sold: {}, Amount Sold: {}, Token Bought: {}, Amount Bought: {}",
                    transaction.version,
                    transaction.sender,
                    transaction.token_sold,
                    transaction.token_sold_amount,
                    transaction.token_bought,
                    transaction.token_bought_amount
                );
            }
        }
        Err(e) => {
            println!("Error fetching transactions: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_get_token_supply() {
    let external = External::new();
    let address = "0x159df6b7689437016108a019fd5bef736bac692b6d4a1f10c941f6fbb9a74ca6";
    let token = "0x159df6b7689437016108a019fd5bef736bac692b6d4a1f10c941f6fbb9a74ca6::oft::CakeOFT";

    match external.get_token_supply(address, token).await {
        Ok(supply) => {
            println!("Token Supply: {}", supply);
        }
        Err(e) => {
            println!("Error fetching token supply: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_calculate_market_cap() {
    if dotenv::dotenv().is_err() {
        println!("Starting server without .env file.");
    }
    let config = crate::Config::init();
    let sqlx_db_connection = database::connect_sqlx(&config.db_url).await;
    let db = database::PostgreDatabase::new(sqlx_db_connection);
    let address = "0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa";
    let token = "0x159df6b7689437016108a019fd5bef736bac692b6d4a1f10c941f6fbb9a74ca6::oft::CakeOFT";
    let token_address = "0x159df6b7689437016108a019fd5bef736bac692b6d4a1f10c941f6fbb9a74ca6";

    let external = External::new();
    match external
        .calculate_market_cap(&db, address, token, token_address)
        .await
    {
        Ok(market_cap) => {
            println!("Fully Diluted Market Cap: {}", market_cap.fully_diluted);
            println!("Normal Market Cap: {}", market_cap.normal);
        }
        Err(e) => {
            println!("Error calculating market cap: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_get_number_of_token_holders() {
    let external = External::new();
    let token = "0x159df6b7689437016108a019fd5bef736bac692b6d4a1f10c941f6fbb9a74ca6::oft::CakeOFT";

    match external.get_number_of_token_holders(token).await {
        Ok(count) => {
            println!("Number of token holders: {}", count);
        }
        Err(e) => {
            println!("Error getting number of token holders: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_calculate_trading_volume() {
    // Initialize the External struct
    let external = External::new();

    // Set up test parameters
    let address = "0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa";
    let entry_function_id = "0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa::router::swap_exact_input";

    // Call the calculate_trading_volume function
    match external
        .calculate_trading_volume(address, entry_function_id)
        .await
    {
        Ok(volume) => {
            println!("Successful calculation:");
            println!("Total trading volume in the last 7 days: ${:.2}", volume);
        }
        Err(e) => {
            println!("Error occurred during calculation:");
            println!("Error: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_get_daily_active_users() {
    let external = External::new();
    let address = "0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa";

    match external.get_daily_active_users(address).await {
        Ok(count) => println!("Number of daily active users: {}", count),
        Err(e) => eprintln!("Error: {}", e),
    }
}

#[tokio::test]
async fn test_get_weekly_active_users() {
    let external = External::new();
    let address = "0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa";

    match external.get_weekly_active_users(address).await {
        Ok(count) => println!("Number of weekly active users: {}", count),
        Err(e) => eprintln!("Error: {}", e),
    }
}

#[tokio::test]
async fn test_get_fee_7d_pancake() {
    let external = External::new();

    match external.get_fee_within_n_days_pancake(7).await {
        Ok(count) => println!("Fee (7d) of pancake: {}", count),
        Err(e) => eprintln!("Error: {}", e),
    }
}

#[tokio::test]
async fn test_get_fee_30d_pancake() {
    let external = External::new();

    match external.get_fee_within_n_days_pancake(30).await {
        Ok(count) => println!("Fee (30d) of pancake: {}", count),
        Err(e) => eprintln!("Error: {}", e),
    }
}
