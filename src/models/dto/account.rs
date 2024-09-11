use serde::{Deserialize, Serialize};
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
    pub entity_id: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateAccount {
    pub entity_id: Option<i32>,
}
