use std::collections::HashMap;
use serde_json::Value;
use reqwest::Client;

const FULLNODE_API: &str = "https://api.mainnet.aptoslabs.com/v1/";
pub const USDT: &str = "0xf22bede237a07e121b56d91a491eb7bcdfd1f5907926a9e58338f964a01b17fa::asset::USDT";
pub const USDC: &str = "0xf22bede237a07e121b56d91a491eb7bcdfd1f5907926a9e58338f964a01b17fa::asset::USDC";
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

    pub async fn get_total_value_locked(&self, address: &str) -> Result<f64, reqwest::Error> {
        let res: Value = self
            .client
            .get(format!("{FULLNODE_API}accounts/{address}/resources"))
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
                if let Some((price, decimals)) = External::get_price_and_decimals(client, &token_clone).await {
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
            let price = balance_y / balance_x * 10f64.powi(decimals as i32 - DECIMALS_USD as i32);
            return Some((price, decimals));
        }

        if let Some((balance_x, balance_y)) = usdt_result {
            let price = balance_y / balance_x * 10f64.powi(decimals as i32 - DECIMALS_USD as i32);
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
            .post(format!("{FULLNODE_API}graphql"))
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

    async fn get_balances(client: &Client, token: &str, stablecoin: &str) -> Option<(f64, f64)> {
        let response: Value = client
            .get(format!(
                "{FULLNODE_API}accounts/0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa/resource/0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa::swap::TokenPairMetadata<{},{}>",
                token, stablecoin
            ))
            .send()
            .await
            .ok()?
            .json()
            .await
            .ok()?;

        let balance_x: f64 = response["data"]["balance_x"]["value"]
            .as_str()?
            .parse()
            .ok()?;
        let balance_y: f64 = response["data"]["balance_y"]["value"]
            .as_str()?
            .parse()
            .ok()?;

        Some((balance_x, balance_y))
    }
}
