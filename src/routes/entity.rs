use std::sync::Arc;

use crate::{
    models::{
        dto::{CreateEntityInfo, EntityResponse},
        Entity, Error,
    },
    AppState,
};
use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    routing::{get, post},
    Json, Router,
};
use utoipa::OpenApi;

use super::middlewares::auth_guard;
#[derive(OpenApi)]
#[openapi(paths(create_entity_handler, get_entity_handler))]
/// Defines the OpenAPI spec for entity endpoints
pub struct EntityApi;

/// Used to group entity endpoints together in the OpenAPI documentation
pub const ENTITY_API_GROUP: &str = "ENTITY";

/// Builds a router for all the entity routes
pub fn entity_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_entity_handler))
        .route("/:id", get(get_entity_handler))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_guard))
}
#[utoipa::path(
    post,
    path = "/api/entity",
    tag = ENTITY_API_GROUP,
    request_body = CreateEntityInfo,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 201, description = "Entity successfully created", body = EntityResponse),
        (status = 400, description = "Bad request"),
    )
)]
pub async fn create_entity_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateEntityInfo>,
) -> Result<Json<EntityResponse>, Error> {
    let new_entity = Entity {
        name: body.name,
        ..Default::default()
    };

    let entity = state.db.create_entity(&new_entity).await?;
    Ok(Json(EntityResponse {
        id: entity.id,
        name: entity.name,
        created_at: entity.created_at.to_string(),
        updated_at: entity.updated_at.to_string(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/entity/{id}",
    tag = ENTITY_API_GROUP,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Entity found", body = EntityResponse),
        (status = 404, description = "Entity not found"),
    ),
    params(
        ("id" = i32, Path, description = "Entity ID")
    )
)]
pub async fn get_entity_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i32>,
) -> Result<Json<EntityResponse>, Error> {
    let entity = state.db.get_entity_by_id(id).await?;
    let entity = entity.ok_or((StatusCode::NOT_FOUND, "Entity not found"))?;

    Ok(Json(EntityResponse {
        id: entity.id,
        name: entity.name,
        created_at: entity.created_at.to_string(),
        updated_at: entity.updated_at.to_string(),
    }))
}
