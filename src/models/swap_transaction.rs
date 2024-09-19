use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct SwapTransaction {
    pub version: i64,
    pub sender: String,
    pub token_sold: String,
    pub token_sold_amount: f64,
    pub token_bought: String,
    pub token_bought_amount: f64,
}
