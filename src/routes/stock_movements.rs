use axum::{
    routing::{get, post},
    Router,
};
use crate::state::AppState;
use crate::handlers::stock_movement;
use crate::middleware::auth::require_auth;

pub fn routes() -> Router<AppState> {
    Router::new()
        // All routes require authentication
        .route("/stock-movements/batches/{batch_id}", get(stock_movement::get_batch_movements))
        .route("/stock-movements/daily/{date}", get(stock_movement::get_daily_movements))
        .route("/stock-movements/products/{product_id}", get(stock_movement::get_product_movements))
        .route("/stock-movements/adjust", post(stock_movement::create_stock_adjustment))
        .route_layer(axum::middleware::from_fn(require_auth))
}
