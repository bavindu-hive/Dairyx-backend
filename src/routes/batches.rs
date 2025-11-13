use axum::{
    routing::get,
    Router,
};
use crate::state::AppState;
use crate::handlers::batch::{list_batches, get_batch};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/batches", get(list_batches))
        .route("/batches/{id}", get(get_batch))
}
