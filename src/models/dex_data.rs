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

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct TokenTerminalData {
    pub ath: String,
    pub ath_last: String,
    pub atl: String,
    pub atl_last: String,
    pub revenue_30d: String,
    pub revenue_annualized: String,
    pub expenses_30d: String,
    pub earnings_30d: String,
    pub fees_30d: String,
    pub fees_annualized: String,
    pub token_incentives_30d: String,
    pub monthly_active_users: String,
    pub afpu: String,
    pub arpu: String,
    pub token_trading_volume_30d: String,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct MarketCap {
    pub fully_diluted: f64,
    pub normal: f64,
}
