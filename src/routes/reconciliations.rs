use axum::{
    routing::{get, post},
    Router,
};
use crate::state::AppState;
use crate::handlers::reconciliation;
use crate::middleware::auth::require_auth;

pub fn routes() -> Router<AppState> {
    Router::new()
        // All routes require authentication (manager only)
        .route("/reconciliations/start", post(reconciliation::start_reconciliation))
        .route("/reconciliations", get(reconciliation::list_reconciliations))
        .route("/reconciliations/{date}", get(reconciliation::get_reconciliation))
        .route("/reconciliations/{date}/trucks/{truck_id}/verify", post(reconciliation::verify_truck_return))
        .route("/reconciliations/{date}/finalize", post(reconciliation::finalize_reconciliation))
        .route_layer(axum::middleware::from_fn(require_auth))
}
