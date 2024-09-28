use std::sync::Arc;

use axum::{extract::State, http::StatusCode, middleware, routing::get, Json, Router};
use utoipa::OpenApi;

use crate::{
    external::External,
    models::{
        dto::{CoinPriceQuery, CoinPriceResponse},
        Error,
    },
    AppState,
};

use super::middlewares::auth_guard;

/// Defines the OpenAPI spec for utility endpoints
#[derive(OpenApi)]
#[openapi(paths(get_price_of_coin_handler))]
pub struct UtilsApi;

/// Used to group utility endpoints together in the OpenAPI documentation
pub const UTILS_API_GROUP: &str = "UTILS";

/// Builds a router for utility routes
pub fn utils_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/price", get(get_price_of_coin_handler))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_guard))
}

/// Get price of coin handler function
#[utoipa::path(
    get,
    path = "/api/utils/price",
    tag = UTILS_API_GROUP,
    params(
        ("coin_type" = String, Query, description = "The type of coin to get the price for")
    ),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Price of coin retrieved successfully", body = CoinPriceResponse),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Price not found for the given coin type"),
    )
)]
pub async fn get_price_of_coin_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<CoinPriceQuery>,
) -> Result<Json<CoinPriceResponse>, Error> {
    let (price, decimals) =
        External::get_price_and_decimals(state.ext.client.clone(), &query.coin_type)
            .await
            .ok_or_else(|| {
                Error::new(StatusCode::NOT_FOUND, "Unable to get price for coin type")
            })?;
    Ok(Json(CoinPriceResponse { price, decimals }))
}
