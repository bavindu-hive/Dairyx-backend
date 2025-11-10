use axum::{response::{Response, IntoResponse}};
use axum::http::StatusCode;
use axum::middleware::Next;
use crate::auth::jwt::verify_token;
use serde::Serialize;

#[derive(Clone)]
pub struct AuthContext {
    pub user_id: i64,
    pub role: String,
    pub username: String,
}

#[derive(Serialize)]
struct ErrorBody { error: String, code: &'static str }

use axum::http::Request;

pub async fn require_auth(mut req: Request<axum::body::Body>, next: Next) -> Response {
    let auth_header = match req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok()) {
        Some(h) => h,
        None => return unauthorized("Missing Authorization header"),
    };

    // Expect "Bearer <token>"
    let token = match auth_header.strip_prefix("Bearer ") {
        Some(t) => t,
        None => return unauthorized("Invalid Authorization format"),
    };

    let secret = match std::env::var("JWT_SECRET") {
        Ok(s) => s,
        Err(_) => return unauthorized("Server auth misconfiguration"),
    };

    let claims = match verify_token(token, &secret) {
        Ok(c) => c,
        Err(e) => return unauthorized(&format!("{e:?}")),
    };

    // Attach context
    req.extensions_mut().insert(AuthContext {
        user_id: claims.sub,
        role: claims.role,
        username: claims.username,
        
    });

    next.run(req).await
}

fn unauthorized(msg: &str) -> Response {
    let body = axum::Json(ErrorBody { error: msg.to_string(), code: "unauthorized" });
    (StatusCode::UNAUTHORIZED, body).into_response()
}