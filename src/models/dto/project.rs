use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

use crate::models::Project;

use super::DexProjectResponse;
#[derive(Debug, ToSchema)]
pub enum ProjectResponse {
    Basic(BasicProjectResponse),
    Dex(DexProjectResponse),
}

impl Serialize for ProjectResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ProjectResponse::Basic(basic) => basic.serialize(serializer),
            ProjectResponse::Dex(dex) => dex.serialize(serializer),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct NewProject {
    pub name: String,
    pub token: String,
    pub category: String,
    pub contract_address: Option<String>,
    #[schema(additional_properties)]
    pub attributes: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateProject {
    pub name: Option<String>,
    pub token: Option<String>,
    pub category: Option<String>,
    pub contract_address: Option<String>,
    #[schema(additional_properties)]
    pub attributes: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BasicProjectResponse {
    pub id: i32,
    pub name: String,
    pub token: String,
    pub category: String,
    pub contract_address: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Project> for BasicProjectResponse {
    fn from(project: Project) -> Self {
        BasicProjectResponse {
            id: project.id,
            name: project.name,
            token: project.token,
            category: project.category,
            contract_address: project.contract_address,
            created_at: project.created_at.to_string(),
            updated_at: project.updated_at.to_string(),
        }
    }
}
