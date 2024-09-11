use std::sync::Arc;

use axum::{
    extract::State, http::StatusCode, middleware, response::IntoResponse, routing::{get, post, put}, Json, Router
};
use utoipa::OpenApi;

use crate::{
    models::{
        dto::{ProjectResponse, NewProject, UpdateProject},
        Project, Error
    },
    AppState
};

use super::middlewares::auth_guard;

/// Defines the OpenAPI spec for project endpoints
#[derive(OpenApi)]
#[openapi(paths(create_project_handler, get_project_handler, update_project_handler))]
pub struct ProjectsApi;

/// Used to group project endpoints together in the OpenAPI documentation
pub const PROJECT_API_GROUP: &str = "PROJECT";

/// Builds a router for project routes
pub fn project_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_project_handler))
        .route("/:id", get(get_project_handler))
        .route("/:id", put(update_project_handler))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_guard))
}

/// Create project handler function
#[utoipa::path(
    post,
    path = "/api/project",
    tag = PROJECT_API_GROUP,
    request_body = NewProject,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 201, description = "Project successfully created", body = ProjectResponse),
    )
)]
pub async fn create_project_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<NewProject>,
) -> Result<Json<ProjectResponse>, Error> {
    // Check if the account associated with the project exists
    if let Some(ref address) = body.contract_address {
        if state.db.get_account_by_address(address).await?.is_none() {
            return Err(Error::new(StatusCode::BAD_REQUEST, "Account does not exist"));
        }
    }

    // Create the new project
    let new_project = Project {
        token: body.token.clone(),
        category: body.category.clone(),
        contract_address: body.contract_address.clone(),
        ..Default::default()
    };

    let project = state.db.create_project(&new_project).await?;

    Ok(Json(ProjectResponse {
        id: project.id,
        token: project.token,
        category: project.category,
        contract_address: project.contract_address,
        created_at: project.created_at.to_string(),
        updated_at: project.updated_at.to_string(),
    }))
}

/// Get project handler function
#[utoipa::path(
    get,
    path = "/api/project/{id}",
    tag = PROJECT_API_GROUP,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Project found", body = ProjectResponse),
        (status = 404, description = "Project not found"),
    ),
    params(
        ("id" = i32, Path, description = "Project ID")
    )
)]
pub async fn get_project_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i32>,
) -> Result<impl IntoResponse, StatusCode> {
    let project = state
        .db
        .get_project_by_id(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(project) = project {
        Ok((StatusCode::OK, Json(project)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Update project handler function
#[utoipa::path(
    put,
    path = "/api/project/{id}",
    tag = PROJECT_API_GROUP,
    request_body = UpdateProject,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Project successfully updated", body = ProjectResponse),
        (status = 404, description = "Project not found"),
        (status = 400, description = "Invalid account ID"),
    ),
    params(
        ("id" = i32, Path, description = "Project ID")
    )
)]
pub async fn update_project_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i32>,
    Json(body): Json<UpdateProject>,
) -> Result<impl IntoResponse, Error> {
    // Fetch the project by ID
    let project = state
        .db
        .get_project_by_id(id)
        .await
        .map_err(|_| Error::new(StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch project"))?;

    if let Some(mut project) = project {
        // Check if the contract_address is provided and exists
        if let Some(address) = body.contract_address {
            if state.db.get_account_by_address(&address).await?.is_none() {
                return Err(Error::new(StatusCode::BAD_REQUEST, "Account does not exist"));
            }
            project.contract_address = Some(address);
        } else {
            project.contract_address = None;
        }

        // Update the fields if they are provided
        if let Some(token) = body.token {
            project.token = token;
        }

        if let Some(category) = body.category {
            project.category = category;
        }

        // Persist the updated project to the database
        let updated_project = state.db.update_project(&project).await?;

        Ok(Json(ProjectResponse {
            id: updated_project.id,
            token: updated_project.token,
            category: updated_project.category,
            contract_address: updated_project.contract_address,
            created_at: updated_project.created_at.to_string(),
            updated_at: updated_project.updated_at.to_string(),
        }))
    } else {
        Err(Error::new(StatusCode::NOT_FOUND, "Project not found"))
    }
}
