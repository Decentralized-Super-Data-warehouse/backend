pub mod message;
pub mod user;
pub mod entity;
pub mod account;
pub mod project;
pub mod dex_data;
pub mod utils;
pub use message::Message;
pub use user::*;
pub use entity::*;
pub use account::*;
pub use project::*;
pub use dex_data::*;
pub use utils::*;

use utoipa::{
    openapi::security::{Http, HttpAuthScheme, SecurityScheme},
    Modify, OpenApi,
};

use crate::models::SwapTransaction;
#[derive(OpenApi)]
#[openapi(
    components(
        schemas(
            Profile,
            LoginInfo,
            RegisterInfo,
            TokenResponse,
            CreateEntityInfo,
            EntityResponse,
            NewAccount,
            UpdateAccount,
            AccountResponse,
            NewProject,
            UpdateProject,
            ProjectResponse,
            BasicProjectResponse,
            DexProjectResponse,
            SwapTransaction,
            AccountDetailsResponse,
            CoinPriceResponse
        ),
    ),     
    modifiers(&SecurityAddon)
)]
/// Captures OpenAPI schemas and canned responses defined in the DTO module
pub struct OpenApiSchemas;

pub struct SecurityAddon;
impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components: &mut utoipa::openapi::Components = openapi.components.as_mut().unwrap(); // we can unwrap safely since there already is components registered.
        components.add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        )
    }
}
