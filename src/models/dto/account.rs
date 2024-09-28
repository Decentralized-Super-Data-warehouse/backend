use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct NewAccount {
    pub address: String,
    pub entity_id: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AccountResponse {
    pub id: i32,
    pub address: String,
    pub name: Option<String>,
    pub entity_id: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateAccount {
    pub name: Option<String>,
    pub entity_id: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AccountDetailsResponse {
    pub name: Option<String>,
    pub category: String,
    pub transactions: Vec<Transaction>,
    pub coins: Vec<Coin>,
}

#[derive(Debug, Serialize)]
pub struct Transaction {
    pub version: u64,
    pub timestamp: String,
    pub sender: String,
    pub receiver: String,
    pub function: String,
    pub amount: u64,
    pub gas_amount: u64,
}

#[derive(Debug, Serialize)]
pub struct Coin {
    pub asset_type: String,
    pub name: String,
    pub symbol: String,
    pub amount: f64,
}

// GraphQL query responses
#[derive(Debug, Deserialize)]
pub struct CoinBalanceResponse {
    pub data: CoinBalanceData,
}

#[derive(Debug, Deserialize)]
pub struct CoinBalanceData {
    pub current_fungible_asset_balances: Vec<FungibleAssetBalance>,
}

#[derive(Debug, Deserialize)]
pub struct FungibleAssetBalance {
    #[serde(deserialize_with = "deserialize_amount_to_u64")]
    pub amount_v1: u64,
    #[serde(deserialize_with = "deserialize_string_or_null")]
    pub asset_type_v1: String,
    pub metadata: FungibleAssetMetadata,
}

#[derive(Debug, Deserialize)]
pub struct FungibleAssetMetadata {
    pub decimals: i32,
    pub name: String,
    pub symbol: String,
}

#[derive(Debug, Deserialize)]
pub struct TransactionResponse {
    pub data: TransactionData,
}

#[derive(Debug, Deserialize)]
pub struct TransactionData {
    pub account_transactions: Vec<AccountTransaction>,
}

#[derive(Debug, Deserialize)]
pub struct AccountTransaction {
    pub transaction_version: u64,
    pub user_transaction: UserTransaction,
    pub coin_activities: Vec<CoinActivity>,
}

#[derive(Debug, Deserialize)]
pub struct UserTransaction {
    pub entry_function_id_str: String,
    pub timestamp: String,
    pub sender: String,
}

#[derive(Debug, Deserialize)]
pub struct CoinActivity {
    pub amount: u64,
    pub coin_type: String,
    pub activity_type: String,
}

fn deserialize_amount_to_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Number(n) => n
            .as_u64()
            .ok_or_else(|| serde::de::Error::custom("Expected u64")),
        Value::Null => Ok(0),
        _ => Err(serde::de::Error::custom("Expected u64 or null")),
    }
}

fn deserialize_string_or_null<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(s) => Ok(s),
        Value::Null => Ok(String::new()),
        _ => Err(serde::de::Error::custom("Expected string or null")),
    }
}
