use axum::{
    routing::{get, post, patch, delete},
    Router,
};
use crate::state::AppState;
use crate::handlers::allowance;
use crate::middleware::auth::require_auth;

pub fn routes() -> Router<AppState> {
    Router::new()
        // All routes require authentication (manager only)
        .route("/allowances", post(allowance::create_allowance))
        .route("/allowances", get(allowance::list_allowances))
        .route("/allowances/{id}", get(allowance::get_allowance))
        .route("/allowances/{id}", delete(allowance::delete_allowance))
        .route("/allowances/{id}/allocate", post(allowance::allocate_to_trucks))
        .route("/allowances/{id}/trucks/{truck_id}", patch(allowance::update_truck_allocation))
        .route("/allowances/{id}/finalize", post(allowance::finalize_allowance))
        .route_layer(axum::middleware::from_fn(require_auth))
}
