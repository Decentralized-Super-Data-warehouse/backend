use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    routing::{get, post, put},
    Json, Router,
};
use utoipa::OpenApi;

use crate::{
    models::{
        dto::{
            BasicProjectResponse, DexProjectResponse, NewProject, ProjectResponse, UpdateProject,
        },
        project::{Project, ProjectAttribute},
        Error,
    },
    AppState,
};

use super::middlewares::auth_guard;

/// Defines the OpenAPI spec for project endpoints
#[derive(OpenApi)]
#[openapi(paths(
    create_project_handler,
    get_project_handler,
    get_project_by_name_handler,
    update_project_handler
))]
pub struct ProjectsApi;

/// Used to group project endpoints together in the OpenAPI documentation
pub const PROJECT_API_GROUP: &str = "PROJECT";

/// Builds a router for project routes
pub fn project_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_project_handler))
        .route("/:id", get(get_project_handler))
        .route("/name/:name", get(get_project_by_name_handler))
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
) -> Result<Json<BasicProjectResponse>, Error> {
    // Check if the account associated with the project exists
    if let Some(ref address) = body.contract_address {
        if state.db.get_account_by_address(address).await?.is_none() {
            return Err(Error::new(
                StatusCode::BAD_REQUEST,
                "Account does not exist",
            ));
        }
    }

    // Create the new project
    let new_project = Project {
        name: body.name.clone(),
        token: body.token.clone(),
        category: body.category.clone(),
        contract_address: body.contract_address.clone(),
        attributes: body
            .attributes
            .iter()
            .map(|(key, value)| ProjectAttribute {
                key: key.clone(),
                value: value.clone(),
            })
            .collect(),
        ..Default::default()
    };

    let project = state.db.create_project(&new_project).await?;

    Ok(Json(BasicProjectResponse::from(project)))
}

/// Get project by ID handler function
#[utoipa::path(
    get,
    path = "/api/project/{id}",
    tag = PROJECT_API_GROUP,
    security(
        ("bearerAuth" = [])
    ),
    params(
        ("id" = i32, Path, description = "The ID of the project to fetch")
    ),
    responses(
        (status = 200, description = "Project successfully fetched", body = ProjectResponse),
        (status = 404, description = "Project not found"),
    )
)]
pub async fn get_project_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i32>,
) -> Result<Json<BasicProjectResponse>, Error> {
    if let Some(project) = state.db.get_project_by_id(id).await? {
        Ok(Json(BasicProjectResponse::from(project)))
    } else {
        Err(Error::new(StatusCode::NOT_FOUND, "Project not found"))
    }
}

/// Get project by name handler function
#[utoipa::path(
    get,
    path = "/api/project/name/{name}",
    security(
        ("bearerAuth" = [])
    ),
    tag = PROJECT_API_GROUP,
    params(
        ("name" = String, Path, description = "The name of the project to fetch")
    ),
    responses(
        (status = 200, description = "Project successfully fetched", body = ProjectResponse),
        (status = 404, description = "Project not found"),
    )
)]
pub async fn get_project_by_name_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<ProjectResponse>, Error> {
    if let Some(project) = state.db.get_project_by_name(&name).await? {
        match project.category.as_str() {
            "DEX" => {
                if let (Some(contract_address), Some(entry_function_id_str)) = (
                    &project.contract_address,
                    project.get_string("entry_function_id_str"),
                ) {
                    let transactions = state
                        .ext
                        .get_swap_transactions(contract_address, &entry_function_id_str)
                        .await?;

                    // Create DexProjectResponse
                    let dex_response = DexProjectResponse::from_project(project, transactions)
                        .ok_or_else(|| {
                            Error::new(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to create DexProjectResponse",
                            )
                        })?;

                    Ok(Json(ProjectResponse::Dex(dex_response)))
                } else {
                    Err(Error::new(
                        StatusCode::BAD_REQUEST,
                        "Missing contract_address or entry_function_id_str in project attributes",
                    ))
                }
            }
            _ => Err(Error::new(
                StatusCode::BAD_REQUEST,
                "Unknown project category",
            )),
        }
    } else {
        Err(Error::new(StatusCode::NOT_FOUND, "Project not found"))
    }
}

/// Update project handler function
#[utoipa::path(
    put,
    path = "/api/project/{id}",
    tag = PROJECT_API_GROUP,
    params(
        ("id" = i32, Path, description = "The ID of the project to update")
    ),
    request_body = UpdateProject,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Project successfully updated", body = ProjectResponse),
        (status = 404, description = "Project not found"),
    )
)]
pub async fn update_project_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i32>,
    Json(body): Json<UpdateProject>,
) -> Result<Json<BasicProjectResponse>, Error> {
    // Fetch the existing project
    let mut project = if let Some(project) = state.db.get_project_by_id(id).await? {
        project
    } else {
        return Err(Error::new(StatusCode::NOT_FOUND, "Project not found"));
    };

    // Update fields
    if let Some(name) = body.name {
        project.name = name;
    }
    if let Some(token) = body.token {
        project.token = token;
    }
    if let Some(category) = body.category {
        project.category = category;
    }
    project.contract_address = body.contract_address;

    // Update attributes
    if let Some(attributes) = body.attributes {
        project.attributes = attributes
            .iter()
            .map(|(key, value)| ProjectAttribute {
                key: key.clone(),
                value: value.clone(),
            })
            .collect();
    }

    let updated_project = state.db.update_project(&project).await?;
    Ok(Json(BasicProjectResponse::from(updated_project)))
}
