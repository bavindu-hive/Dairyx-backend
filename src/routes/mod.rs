pub mod products;
pub mod users;
pub mod deliveries;
pub mod trucks;
pub mod truck_loads;
pub mod shops;
pub mod sales;
pub mod allowances;
pub mod reconciliations;
pub mod stock_movements;
pub mod batches;

use axum::Router;
use crate::state::AppState;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .merge(products::routes())
        .merge(users::routes())
        .merge(deliveries::routes())
        .merge(trucks::routes())
        .merge(truck_loads::routes())
        .merge(shops::routes())
        .merge(sales::routes())
        .merge(allowances::routes())
        .merge(reconciliations::routes())
        .merge(stock_movements::routes())
        .merge(batches::routes())
}