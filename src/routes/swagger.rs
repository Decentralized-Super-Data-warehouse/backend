use crate::models::dto;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(info(
    title = "Rust Decentralized Data Warehouse API",
    description = "Rust Decentralized Data Warehouse API For Aptos Code Collision",
))]
struct Api;

/// Constructs the route on the API that renders the swagger UI and returns the OpenAPI schema.
/// Merges in OpenAPI definitions from other locations in the app, such as the [dto] package
/// and submodules of [api][crate::api]
pub fn build_documentation() -> SwaggerUi {
    let mut api_docs = Api::openapi();
    api_docs.merge(dto::OpenApiSchemas::openapi());
    api_docs.merge(super::health::HealthApi::openapi());
    api_docs.merge(super::user::UsersApi::openapi());
    api_docs.merge(super::entity::EntityApi::openapi());
    api_docs.merge(super::account::AccountsApi::openapi());
    api_docs.merge(super::project::ProjectsApi::openapi());

    SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api_docs)
}
