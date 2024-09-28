use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CoinPriceQuery {
    pub coin_type: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CoinPriceResponse {
    pub decimals: u8,
    pub price: f64,
}
