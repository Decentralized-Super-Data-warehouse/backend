use serde::{Deserialize, Serialize};
use utoipa::ToSchema;


#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateEntityInfo {
    pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EntityResponse {
    pub id: i32,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

