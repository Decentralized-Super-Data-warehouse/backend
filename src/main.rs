mod app_state;
mod config;
mod database;
mod models;
mod routes;
pub mod external;
pub use app_state::AppState;
pub use config::Config;
use external::External;

use crate::routes::make_app;
use dotenv::dotenv;
use std::{error::Error, time::Instant};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let app = make_app().await?;
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("🚀 Server started successfully");
    axum::serve(listener, app).await?;
    Ok(())
}
