pub mod products;
pub mod users;
pub mod deliveries;
pub mod trucks;
pub mod truck_loads;

use axum::Router;
use crate::state::AppState;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .merge(products::routes())
        .merge(users::routes())
        .merge(deliveries::routes())
        .merge(trucks::routes())
        .merge(truck_loads::routes())
}