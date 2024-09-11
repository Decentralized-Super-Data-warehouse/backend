use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use std::sync::Arc;
use utoipa::OpenApi;

use crate::{
    models::{
        dto::{LoginInfo, Profile, RegisterInfo, TokenResponse},
        Error, TokenClaim, User,
    },
    AppState,
};

use super::middlewares::auth_guard;

#[derive(OpenApi)]
#[openapi(paths(login_handler, register_user_handler, get_profile_handler))]
/// Defines the OpenAPI spec for user endpoints
pub struct UsersApi;

/// Used to group user endpoints together in the OpenAPI documentation
pub const USER_API_GROUP: &str = "USER";

/// Builds a router for all the user routes
pub fn user_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/signup", post(register_user_handler))
        .route("/login", post(login_handler))
        .route(
            "/profile",
            get(get_profile_handler)
                .route_layer(middleware::from_fn_with_state(state.clone(), auth_guard)),
        )
}

// Login handler function
#[utoipa::path(
    post,
    path = "/api/user/login",
    tag = USER_API_GROUP,
    request_body = LoginInfo,
    responses(
        (status = 201, description = "User successfully created"),
    )
)]
pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginInfo>,
) -> Result<impl IntoResponse, Error> {
    let user = state.db.get_user_by_email(&body.email).await?;
    let user: User = user.ok_or((StatusCode::BAD_REQUEST, "User does not exist"))?;
    let hash = PasswordHash::new(&user.hashed_password)?;
    Argon2::default().verify_password(body.password.as_bytes(), &hash)?;

    let now = Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + Duration::days(7)).timestamp() as usize;

    let claims = TokenClaim {
        sub: user.email,
        exp,
        iat,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.jwt_secret.as_ref()),
    )?;

    Ok(Json(TokenResponse { token }))
}

// Register user handler function
#[utoipa::path(
    post,
    path = "/api/user/signup",
    tag = USER_API_GROUP,
    request_body = RegisterInfo,
    responses(
        (status = 201, description = "User successfully created", body = Profile),
    )
)]
pub async fn register_user_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegisterInfo>,
) -> Result<impl IntoResponse, Error> {
    if state.db.get_user_by_email(&body.email).await?.is_some() {
        return Err(Error::new(StatusCode::BAD_REQUEST, "Email already exists"));
    }

    let salt = SaltString::generate(&mut OsRng);
    let hashed_password = Argon2::default()
        .hash_password(body.password.as_bytes(), &salt)?
        .to_string();

    let data = User {
        name: body.name,
        email: body.email.to_ascii_lowercase(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        hashed_password,
        ..Default::default()
    };

    let user: User = state.db.create_user(&data).await?;
    Ok(Json(Profile::from(user)))
}

// Get profile handler function
#[utoipa::path(
    get,
    path = "/api/user/profile",
    tag = USER_API_GROUP,
    responses(
        (status = 200, description = "User profile successfully retrieved", body = Profile),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn get_profile_handler(Extension(user): Extension<User>) -> impl IntoResponse {
    Json(Profile::from(user))
}
