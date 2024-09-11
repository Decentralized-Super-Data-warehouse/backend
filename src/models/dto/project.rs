use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct NewProject {
    pub token: String,
    pub category: String,
    pub contract_address: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProject {
    pub token: Option<String>,
    pub category: Option<String>,
    pub contract_address: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProjectResponse {
    pub id: i32,
    pub token: String,
    pub category: String,
    pub contract_address: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
