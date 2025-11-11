use axum::{
    routing::{get, post},
    Router, middleware,
};
use crate::state::AppState;
use crate::handlers::shop::{create_shop, get_shop, list_shops, update_shop, delete_shop};
use crate::middleware::auth::require_auth;

pub fn routes() -> Router<AppState> {
    // All shop viewing is open (drivers and managers can view)
    let open_routes = Router::new()
        .route("/shops", get(list_shops))
        .route("/shops/{id}", get(get_shop));

    // Only managers can create, update, delete
    let protected_routes = Router::new()
        .route("/shops", post(create_shop))
        .route("/shops/{id}", axum::routing::put(update_shop))
        .route("/shops/{id}", axum::routing::delete(delete_shop))
        .layer(middleware::from_fn(require_auth));

    open_routes.merge(protected_routes)
}
