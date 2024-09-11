use crate::models::dto::Message;
use axum::{response::IntoResponse, Json};
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(paths(
    health_checker_handler
))]
/// Defines the OpenAPI spec for user endpoints
pub struct HealthApi;
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "HEALTH",
    responses(
        (status = OK, description = "Success", body = str, content_type = "text/plain")
    )
)]
pub async fn health_checker_handler() -> impl IntoResponse {
    Json(Message::new(
        "OK, I'm alive!",
    ))
}
