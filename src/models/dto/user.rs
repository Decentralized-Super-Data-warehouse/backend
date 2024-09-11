use crate::models::User;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct Profile {
    pub name: String,
    pub email: String,
    #[schema(example = "ADMIN")]
    pub role: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TokenResponse {
    pub token: String,
}

impl From<User> for Profile {
    fn from(user: User) -> Self {
        Self {
            email: user.email.to_owned(),
            name: user.name.to_owned(),
            role: user.role.to_owned(),
            created_at: user.created_at.to_string(),
            updated_at: user.updated_at.to_string(),
        }
    }
}
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginInfo {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterInfo {
    pub name: String,
    pub email: String,
    pub password: String,
}
