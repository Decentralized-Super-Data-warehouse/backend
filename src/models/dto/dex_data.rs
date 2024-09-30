use crate::models::{Project, SwapTransaction};
use serde::Serialize;
use utoipa::ToSchema;

use super::BasicProjectResponse;

#[derive(Debug, Serialize, ToSchema)]
pub struct DexProjectResponse {
    #[serde(flatten)]
    pub base: BasicProjectResponse,
    pub num_chains: Option<i32>,
    pub core_developers: Option<i32>,
    pub code_commits: Option<i32>,
    pub total_value_locked: Option<f64>,
    pub token_max_supply: Option<i64>,
    pub ath: Option<String>,
    pub ath_last: Option<String>,
    pub atl: Option<String>,
    pub atl_last: Option<String>,
    pub revenue_30d: Option<String>,
    pub revenue_annualized: Option<String>,
    pub expenses_30d: Option<String>,
    pub earnings_30d: Option<String>,
    pub fees_30d: Option<String>,
    pub fees_annualized: Option<String>,
    pub daily_fees: Option<f64>,
    pub token_incentives_30d: Option<String>,
    pub monthly_active_users: Option<String>,
    pub afpu: Option<String>,
    pub arpu: Option<String>,
    pub token_trading_volume_30d: Option<String>,
    pub market_cap_fully_diluted: Option<f64>,
    pub market_cap_circulating: Option<f64>,
    pub token_supply: Option<f64>,
    pub num_token_holders: Option<i32>,
    pub trading_volume: Option<f64>,
    pub daily_active_users: Option<i32>,
    pub weekly_active_users: Option<i32>,
    pub transactions: Vec<SwapTransaction>,
}

impl DexProjectResponse {
    pub fn from_project(project: Project, transactions: Vec<SwapTransaction>) -> Option<Self> {
        if project.category != "DEX" {
            return None;
        }

        let base = BasicProjectResponse::from(project.clone());

        Some(DexProjectResponse {
            base,
            num_chains: project.get_int("num_chains"),
            core_developers: project.get_int("core_developers"),
            code_commits: project.get_int("code_commits"),
            total_value_locked: project.get_float("total_value_locked"),
            token_max_supply: project.get_int("token_max_supply").map(|i| i as i64),
            ath: project.get_string("ath"),
            ath_last: project.get_string("ath_last"),
            atl: project.get_string("atl"),
            atl_last: project.get_string("atl_last"),
            revenue_30d: project.get_string("revenue_30d"),
            revenue_annualized: project.get_string("revenue_annualized"),
            expenses_30d: project.get_string("expenses_30d"),
            earnings_30d: project.get_string("earnings_30d"),
            fees_30d: project.get_string("fees_30d"),
            fees_annualized: project.get_string("fees_annualized"),
            daily_fees: project.get_float("daily_fees"),
            token_incentives_30d: project.get_string("token_incentives_30d"),
            monthly_active_users: project.get_string("monthly_active_users"),
            afpu: project.get_string("afpu"),
            arpu: project.get_string("arpu"),
            token_trading_volume_30d: project.get_string("token_trading_volume_30d"),
            market_cap_fully_diluted: project.get_float("market_cap_fully_diluted"),
            market_cap_circulating: project.get_float("market_cap_circulating"),
            token_supply: project.get_float("token_supply"),
            num_token_holders: project.get_int("num_token_holders"),
            trading_volume: project.get_float("trading_volume"),
            daily_active_users: project.get_int("daily_active_users"),
            weekly_active_users: project.get_int("weekly_active_users"),
            transactions,
        })
    }
}
