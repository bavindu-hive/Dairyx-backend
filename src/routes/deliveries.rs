use axum::{Router, routing::{get, post, put, delete}, middleware};
use crate::state::AppState;
use crate::handlers::delivery::{
    create_delivery, get_delivery, list_deliveries, update_delivery, delete_delivery,
};
use crate::middleware::auth::require_auth;

pub fn routes() -> Router<AppState> {
    // Public endpoints: list + get
    let open = Router::new()
        .route("/deliveries", get(list_deliveries))
        .route("/deliveries/{id}", get(get_delivery));

    // Protected endpoints: create/update/delete (JWT required)
    let protected = Router::new()
        .route("/deliveries", post(create_delivery))
        .route("/deliveries/{id}", put(update_delivery).delete(delete_delivery))
        .layer(middleware::from_fn(require_auth));

    open.merge(protected)
}
