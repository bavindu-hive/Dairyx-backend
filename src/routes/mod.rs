pub mod products;
pub mod users;
pub mod deliveries;

use axum::Router;
use crate::state::AppState;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .merge(products::routes())
        .merge(users::routes())
        .merge(deliveries::routes())
}