use axum::{Router, routing::{post, get}, middleware};
use crate::state::AppState;
use crate::handlers::user::{register_user, login_user, get_me};
use crate::middleware::auth::require_auth;

pub fn routes() -> Router<AppState> {
    let open = Router::new()
        .route("/users/register", post(register_user))
        .route("/users/login", post(login_user));

    let protected = Router::new()
        .route("/users/me", get(get_me))
        .layer(middleware::from_fn(require_auth));

    open.merge(protected)
}