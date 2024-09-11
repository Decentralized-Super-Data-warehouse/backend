use std::sync::Arc;

use axum::{
    extract::State, http::StatusCode, middleware, response::IntoResponse, routing::{get, post, put}, Json, Router
};
use utoipa::OpenApi;

use crate::{models::{dto::{AccountResponse, NewAccount, UpdateAccount}, Account, Error}, AppState};

use super::middlewares::auth_guard;

/// Defines the OpenAPI spec for account endpoints
#[derive(OpenApi)]
#[openapi(paths(create_account_handler, get_account_handler, update_account_handler))]
pub struct AccountsApi;

/// Used to group entity endpoints together in the OpenAPI documentation
pub const ACCOUNT_API_GROUP: &str = "ACCOUNT";

/// Builds a router for account routes
pub fn account_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_account_handler))
        .route("/:id", get(get_account_handler))
        .route("/:id", put(update_account_handler))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_guard))
}

/// Create account handler function
#[utoipa::path(
    post,
    path = "/api/account",
    tag = ACCOUNT_API_GROUP,
    request_body = NewAccount,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 201, description = "Account successfully created", body = AccountResponse),
    )
)]
pub async fn create_account_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<NewAccount>,
) -> Result<Json<AccountResponse>, Error> {
    // Check if account with the same address already exists
    if state
        .db
        .get_account_by_address(&body.address)
        .await?
        .is_some()
    {
        return Err(Error::new(
            StatusCode::BAD_REQUEST,
            "Account address already exists",
        ));
    }

    // Check if the entity associated with the account exists
    if let Some(entity_id) = body.entity_id { 
        if state.db.get_entity_by_id(entity_id).await?.is_none() {
            return Err(Error::new(StatusCode::BAD_REQUEST, "Entity does not exist"));
        }
    }

    // Create the new account
    let new_account = Account {
        address: body.address.clone(),
        entity_id: body.entity_id,
        ..Default::default()
    };

    let account = state.db.create_account(&new_account).await?;

    Ok(Json(AccountResponse {
        id: account.id,
        address: account.address,
        entity_id: account.entity_id,
        created_at: account.created_at.to_string(),
        updated_at: account.updated_at.to_string(),
    }))
}

/// Get account handler function
#[utoipa::path(
    get,
    path = "/api/account/{id}",
    tag = ACCOUNT_API_GROUP,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Account found", body = AccountResponse),
        (status = 404, description = "Account not found"),
    ),
    params(
        ("id" = i32, Path, description = "Account ID")
    )
)]
pub async fn get_account_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i32>,
) -> Result<impl IntoResponse, StatusCode> {
    let account = state
        .db
        .get_account_by_id(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(account) = account {
        Ok((StatusCode::OK, Json(account)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Update account handler function
#[utoipa::path(
    put,
    path = "/api/account/{id}",
    tag = ACCOUNT_API_GROUP,
    request_body = UpdateAccount,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Account successfully updated", body = AccountResponse),
        (status = 404, description = "Account not found"),
        (status = 400, description = "Invalid entity ID"),
    ),
    params(
        ("id" = i32, Path, description = "Account ID")
    )
)]
pub async fn update_account_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i32>,
    Json(body): Json<UpdateAccount>,
) -> Result<impl IntoResponse, Error> {
    // Fetch the account by ID
    let account = state
        .db
        .get_account_by_id(id)
        .await
        .map_err(|_| Error::new(StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch account"))?;

    if let Some(mut account) = account {
        // Check if the entity_id is provided
        if let Some(entity_id) = body.entity_id {
            // If entity_id is Some(value), check if it exists
            if state.db.get_entity_by_id(entity_id).await?.is_none() {
                return Err(Error::new(StatusCode::BAD_REQUEST, "Entity does not exist"));
            }
            // Update the entity_id to the provided valid value
            account.entity_id = Some(entity_id);
        } else {
            // If entity_id is None, set the account's entity_id to null
            account.entity_id = None;
        }

        // Persist the updated account to the database
        let updated_account = state.db.update_account(&account).await?;

        Ok(Json(AccountResponse {
            id: updated_account.id,
            address: updated_account.address,
            entity_id: updated_account.entity_id,
            created_at: updated_account.created_at.to_string(),
            updated_at: updated_account.updated_at.to_string(),
        }))
    } else {
        Err(Error::new(StatusCode::NOT_FOUND, "Account not found"))
    }
}
