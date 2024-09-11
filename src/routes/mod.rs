mod health;
mod middlewares;
mod swagger;
mod user;
mod entity;
mod account;
mod project;
use crate::database;
use health::health_checker_handler;
use tracing::info;
use tower_http::trace::TraceLayer;

use crate::{AppState, Config};

use axum::{routing::get, Router};
use std::error::Error;
use std::sync::Arc;

pub async fn make_app() -> Result<Router, Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    let config = Config::init();
    // configure_logger(&config.log_level);
    info!("Connecting to PostgreSQL...");
    let sqlx_db_connection = database::connect_sqlx(&config.db_url).await;
    info!("Connected to PostgreSQL!");
    //let cors = HeaderValue::from_str(&config.cors_url)?;
    // TODO: Consider readding CORS here
    //let cors = CorsLayer::new()
    //    .allow_origin(cors)
    //    .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
    //    .allow_credentials(true)
    //    .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);

    let db = database::PostgreDatabase::new(sqlx_db_connection);
    let state = Arc::new(AppState { db, config });
    let ret = Router::new()
        .route("/api", get(health_checker_handler))
        .route("/api/health", get(health_checker_handler))
        .nest("/api/user", user::user_routes(state.clone()))
        .nest("/api/entity", entity::entity_routes(state.clone()))
        .nest("/api/account", account::account_routes(state.clone()))
        .nest("/api/project", project::project_routes(state.clone()))
        .merge(swagger::build_documentation())
        .with_state(state)
        .layer(TraceLayer::new_for_http());
    //.layer(cors);

    Ok(ret)
}
