use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug)]
pub enum AppError {
    Validation(String),
    Conflict(String),
    NotFound(String),
    Db(sqlx::Error),
    Internal(String),
}

impl AppError {
    pub fn validation(msg: impl Into<String>) -> Self { Self::Validation(msg.into()) }
    pub fn conflict(msg: impl Into<String>) -> Self { Self::Conflict(msg.into()) }
    pub fn not_found(msg: impl Into<String>) -> Self { Self::NotFound(msg.into()) }
    pub fn db(e: sqlx::Error) -> Self { Self::Db(e) }
    pub fn internal(msg: impl Into<String>) -> Self { Self::Internal(msg.into()) }
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
    code: &'static str,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg, code) = match self {
            AppError::Validation(m) => (StatusCode::BAD_REQUEST, m, "validation_error"),
            AppError::Conflict(m) => (StatusCode::CONFLICT, m, "conflict"),
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m, "not_found"),
            AppError::Db(e) => {
                // Fallback for unmapped DB errors
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {e}"), "db_error")
            }
            AppError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m, "internal_error"),
        };

        (status, Json(ErrorBody { error: msg, code })).into_response()
    }
}

// Helpful automatic mappings from sqlx errors to friendly responses
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match &e {
            sqlx::Error::RowNotFound => AppError::NotFound("Resource not found".into()),
            sqlx::Error::Database(db_err) => {
                // Postgres SQLSTATE codes
                match db_err.code().as_deref() {
                    Some("23505") => AppError::Conflict("Resource already exists".into()), // unique_violation
                    Some("23503") => AppError::Validation("Invalid reference".into()),      // foreign_key_violation
                    Some("23502") => AppError::Validation("Missing required field".into()), // not_null_violation
                    _ => AppError::Db(e),
                }
            }
            _ => AppError::Db(e),
        }
    }
}