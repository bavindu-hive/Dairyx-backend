use axum::{
    routing::{get, post},
    Router, middleware,
};
use crate::state::AppState;
use crate::handlers::truck_load::{
    create_truck_load, get_truck_load, list_truck_loads, 
    reconcile_truck_load, delete_truck_load
};
use crate::middleware::auth::require_auth;

pub fn routes() -> Router<AppState> {
    let open_routes = Router::new()
        .route("/truck-loads", get(list_truck_loads))
        .route("/truck-loads/{id}", get(get_truck_load));

    let protected_routes = Router::new()
        .route("/truck-loads", post(create_truck_load))
        .route("/truck-loads/{id}/reconcile", axum::routing::put(reconcile_truck_load))
        .route("/truck-loads/{id}", axum::routing::delete(delete_truck_load))
        .layer(middleware::from_fn(require_auth));

    open_routes.merge(protected_routes)
}
