use axum::{
    routing::{get, post, patch},
    Router,
};
use crate::state::AppState;
use crate::handlers::sale;
use crate::middleware::auth::require_auth;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Open routes - anyone can list and view sales
        .route("/sales", get(sale::list_sales).post(sale::create_sale))
        .route("/sales/{id}", get(sale::get_sale))
        .route("/sales/{id}/payment", patch(sale::update_payment))
        .route_layer(axum::middleware::from_fn(require_auth))
}
