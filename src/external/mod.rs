use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use futures::future::join_all;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::Value;
use std::collections::HashMap;
use std::{error::Error, sync::Arc};
use tokio::sync::Mutex;

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

pub struct External {
    client: Client,
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
                        let tokens = obj_type
                            .split("::swap::TokenPairReserve<")
                            .nth(1)
                            .and_then(|s| s.split('>').next())
                            .unwrap_or("")
                            .split(", ");

                        if let Some(data) = obj.get("data").and_then(Value::as_object) {
                            if let (Some(reserve_x), Some(reserve_y)) =
                                (data.get("reserve_x"), data.get("reserve_y"))
                            {
                                if let (Some(reserve_x_str), Some(reserve_y_str)) =
                                    (reserve_x.as_str(), reserve_y.as_str())
                                {
                                    let reserve_x_value = reserve_x_str.parse::<u64>().unwrap_or(0);
                                    let reserve_y_value = reserve_y_str.parse::<u64>().unwrap_or(0);

                                    let mut tokens_iter = tokens.into_iter();
                                    if let Some(token_x) = tokens_iter.next() {
                                        *reserves.entry(token_x.to_string()).or_insert(0) +=
                                            reserve_x_value;
                                    }
                                    if let Some(token_y) = tokens_iter.next() {
                                        *reserves.entry(token_y.to_string()).or_insert(0) +=
                                            reserve_y_value;
                                    }
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

    async fn get_price_and_decimals(client: Client, token: &str) -> Option<(f64, u8)> {
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
    ) -> Result<TokenTerminalData, Box<dyn Error>> {
        // Initialize the browser with headless mode
        let browser = Browser::new(LaunchOptionsBuilder::default().headless(true).build()?)?;

        // Create a new tab and navigate to the project page
        let tab = browser.new_tab()?;
        tab.navigate_to(&format!(
            "https://tokenterminal.com/terminal/projects/{project}"
        ))?;

        // Wait for the page to load (consider using a more robust waiting mechanism)
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        // Get the page content
        let html = tab.get_content()?;
        let document = Html::parse_document(&html);

        // Scrape ATH/ATL data
        let (ath, ath_last, atl, atl_last) = self.scrape_ath_atl(&document)?;

        // Scrape financial data
        let (revenue_30d, revenue_annualized, expenses_30d, earnings_30d) =
            self.scrape_financials(&document)?;

        Ok(TokenTerminalData {
            ath,
            ath_last,
            atl,
            atl_last,
            revenue_30d,
            revenue_annualized,
            expenses_30d,
            earnings_30d,
        })
    }

    fn scrape_ath_atl(
        &self,
        document: &Html,
    ) -> Result<(String, String, String, String), Box<dyn Error>> {
        let span_selector = Selector::parse("span")?;
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

    fn scrape_financials(
        &self,
        document: &Html,
    ) -> Result<(String, String, String, String), Box<dyn Error>> {
        let li_selector = Selector::parse("li")?;
        let div_selector = Selector::parse("div")?;
        let mut revenue_30d = String::new();
        let mut revenue_annualized = String::new();
        let mut expenses_30d = String::new();
        let mut earnings_30d = String::new();

        for li in document.select(&li_selector) {
            let mut divs = li.select(&div_selector);

            if let Some(label_div) = divs.next() {
                let label_text = label_div.text().collect::<String>();

                if label_text.contains("Revenue (30d)") {
                    if let Some(value_div) = divs.next() {
                        revenue_30d = value_div
                            .text()
                            .collect::<Vec<_>>()
                            .first()
                            .cloned()
                            .unwrap_or_default()
                            .to_owned();
                    }
                }

                if label_text.contains("Revenue (annualized)") {
                    if let Some(value_div) = divs.next() {
                        revenue_annualized = value_div
                            .text()
                            .collect::<Vec<_>>()
                            .first()
                            .cloned()
                            .unwrap_or_default()
                            .to_owned();
                    }
                }

                if label_text.contains("Expenses (30d)") {
                    if let Some(value_div) = divs.next() {
                        expenses_30d = value_div
                            .text()
                            .collect::<Vec<_>>()
                            .first()
                            .cloned()
                            .unwrap_or_default()
                            .to_owned();
                    }
                }

                if label_text.contains("Earnings (30d)") {
                    if let Some(value_div) = divs.next() {
                        earnings_30d = value_div
                            .text()
                            .collect::<Vec<_>>()
                            .first()
                            .cloned()
                            .unwrap_or_default()
                            .to_owned();
                    }
                }
            }
        }

        Ok((revenue_30d, revenue_annualized, expenses_30d, earnings_30d))
    }

    /// Get 25 latest transactions impacting PancakeSwap
    pub async fn get_swap_transactions(&self) -> Result<Vec<SwapTransaction>, Box<dyn Error>> {
        let graphql_query = r#"
        query AccountTransactionsData {
            account_transactions(
                limit: 25
                where: {account_address: {_eq: "0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa"}, user_transaction: {entry_function_id_str: {_eq: "0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa::router::swap_exact_input"}}}
                order_by: {transaction_version: desc}
            ) {
                transaction_version
                user_transaction {
                    sender
                }
                coin_activities {
                    activity_type
                    amount
                    coin_type
                    coin_info {
                        decimals
                    }
                }
            }
        }"#;

        let response: Value = self
            .client
            .post(format!("{}/graphql", FULLNODE_API))
            .json(&serde_json::json!({ "query": graphql_query }))
            .send()
            .await?
            .json()
            .await?;

        let mut transactions = Vec::new();

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
    pub async fn get_token_supply(
        &self,
        address: &str,
        token: &str,
    ) -> Result<f64, Box<dyn Error>> {
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

        Err("Failed to get token supply".into())
    }
    pub async fn calculate_market_cap(
        &self,
        db: &database::PostgreDatabase,
        address: &str,
        token: &str,
        token_address: &str,
    ) -> Result<MarketCap, Box<dyn Error>> {
        let client = Client::new();

        // Get the token price
        let price = match Self::get_price_and_decimals(client.clone(), token).await {
            Some((price, _)) => price,
            None => return Err("Failed to get price and decimals".into()),
        };

        // Get the max supply from the database
        let project = db.get_project_by_address(address).await?.unwrap();

        let circulating_supply = self.get_token_supply(token_address, token).await?;

        // Calculate fully diluted and normal market caps
        let fully_diluted = price * (project.token_max_supply.unwrap() as f64);
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
                        left += std::cmp::max(0, i as u64 - 1) * segment;
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
    ) -> Result<f64, Box<dyn Error>> {
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

                                                let amount = activity["amount"]
                                                    .as_u64()
                                                    .unwrap_or(0);
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

                    Ok::<bool, Box<dyn Error + Send + Sync>>(local_found_old_activity)
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
                    Err(e) => return Err(Box::new(e)),
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
                    Err(format!("Failed to get price and decimals of {}", &coin_type))
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
    assert_eq!(result.revenue_30d, "$4.24m");
    assert_eq!(result.revenue_annualized, "$51.54m");
    assert_eq!(result.expenses_30d, "$2.07m");
    assert_eq!(result.earnings_30d, "$2.17m");
}

#[tokio::test]
async fn test_get_swap_transactions() {
    let external = External::new();

    match external.get_swap_transactions().await {
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
