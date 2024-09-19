use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct Project {
    pub id: i32,
    pub token: String,
    pub category: String,
    pub contract_address: Option<String>,
    pub num_chains: Option<i32>,
    pub core_developers: Option<i32>,
    pub code_commits: Option<i32>,
    pub total_value_locked: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
