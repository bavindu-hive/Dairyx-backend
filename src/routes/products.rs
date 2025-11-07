use axum::{
    routing::{get, post, put, delete},
    Router,
};
use crate::handlers::product::{
    get_products, get_product, create_product, update_product, delete_product
};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/products", get(get_products).post(create_product))
    .route("/products/{id}", get(get_product).put(update_product).delete(delete_product))
}