use axum::{
    routing::{get, post},
    Router, middleware,
};
use crate::state::AppState;
use crate::handlers::truck::{create_truck, get_truck, list_trucks, update_truck, delete_truck, update_truck_max_limit};
use crate::middleware::auth::require_auth;

pub fn routes() -> Router<AppState> {
    let open_routes = Router::new()
        .route("/trucks", get(list_trucks))
        .route("/trucks/{id}", get(get_truck));

    let protected_routes = Router::new()
        .route("/trucks", post(create_truck))
        .route("/trucks/{id}", axum::routing::put(update_truck))
        .route("/trucks/{id}", axum::routing::delete(delete_truck))
        .route("/trucks/{id}/max-limit", axum::routing::patch(update_truck_max_limit))
        .layer(middleware::from_fn(require_auth));

    open_routes.merge(protected_routes)
}
