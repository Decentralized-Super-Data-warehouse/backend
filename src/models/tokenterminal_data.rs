use serde::{Deserialize, Serialize};

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
}
