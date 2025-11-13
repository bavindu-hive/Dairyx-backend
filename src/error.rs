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
    Forbidden(String),
    Db(sqlx::Error),
    Internal(String),
}

impl AppError {
    pub fn validation(msg: impl Into<String>) -> Self { Self::Validation(msg.into()) }
    pub fn conflict(msg: impl Into<String>) -> Self { Self::Conflict(msg.into()) }
    pub fn not_found(msg: impl Into<String>) -> Self { Self::NotFound(msg.into()) }
    pub fn forbidden(msg: impl Into<String>) -> Self { Self::Forbidden(msg.into()) }
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
            AppError::Forbidden(m) => (StatusCode::FORBIDDEN, m, "forbidden"),
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
                // Avoid borrowing from temporaries: convert to owned Strings then match on &str
                let code_owned = db_err.code().map(|c| c.to_string());
                let constraint_owned = db_err.constraint().map(|c| c.to_string());
                let code = code_owned.as_deref();
                let constraint = constraint_owned.as_deref();
                match (code, constraint) {
                    (Some("23514"), Some("delivery_items_unit_price_check")) =>
                        AppError::Validation("unit_price must be greater than or equal to 0".into()), // check_violation
                    (Some("23514"), Some("delivery_items_quantity_check")) =>
                        AppError::Validation("quantity must be greater than 0".into()),
                    (Some("23514"), Some("batches_remaining_quantity_check")) =>
                        AppError::Validation("remaining_quantity must be between 0 and quantity".into()),
                    (Some("23514"), Some("batches_check")) =>
                        AppError::Validation("Batch quantity constraint: remaining_quantity must be between 0 and initial quantity".into()),
                    (Some("23514"), Some("shops_distance_check")) =>
                        AppError::Validation("Distance must be greater than or equal to 0".into()),
                    // Sales constraints
                    (Some("23514"), Some("sales_check")) =>
                        AppError::Validation("Sales constraint: amount_paid must be between 0 and total_amount".into()),
                    (Some("23514"), Some("sales_total_amount_check")) =>
                        AppError::Validation("Total amount must be greater than or equal to 0".into()),
                    (Some("23514"), Some("sales_payment_status_check")) =>
                        AppError::Validation("Payment status must be 'paid' or 'pending'".into()),
                    // Sale items constraints
                    (Some("23514"), Some("sale_items_quantity_check")) =>
                        AppError::Validation("Sale item quantity must be greater than 0".into()),
                    (Some("23514"), Some("sale_items_unit_price_check")) =>
                        AppError::Validation("Sale item unit_price must be greater than or equal to 0".into()),
                    (Some("23514"), Some("sale_items_commission_earned_check")) =>
                        AppError::Validation("Commission earned must be greater than or equal to 0".into()),
                    // Truck load items constraints
                    (Some("23514"), Some("truck_load_items_check")) =>
                        AppError::Validation("Truck load constraint: quantity_sold cannot exceed quantity_loaded".into()),
                    (Some("23514"), Some("truck_load_items_check1")) =>
                        AppError::Validation("Truck load constraint: quantity_sold + quantity_returned cannot exceed quantity_loaded".into()),
                    // Reconciliation constraints
                    (Some("23514"), Some("valid_stock_balance")) =>
                        AppError::Validation("Reconciliation balance error: items_loaded must equal (items_sold + items_returned + items_discarded) when verified".into()),
                    (Some("23514"), _) =>
                        AppError::Validation(format!("Constraint violation: {:?}", constraint).into()),

                    // unique_violation
                    (Some("23505"), Some("deliveries_delivery_note_number_key")) =>
                        AppError::Conflict("delivery_note_number must be unique".into()),
                    (Some("23505"), Some("batches_product_id_batch_number_key")) =>
                        AppError::Conflict("Batch number already exists for this product".into()),
                    (Some("23505"), _) =>
                        AppError::Conflict("Resource already exists".into()),

                    // foreign_key_violation
                    (Some("23503"), Some(_)) => AppError::Validation("Invalid reference".into()),
                    (Some("23503"), None) => AppError::Validation("Invalid reference".into()),

                    // not_null_violation
                    (Some("23502"), _) => AppError::Validation("Missing required field".into()),

                    // invalid_text_representation
                    (Some("22P02"), _) => AppError::Validation("Invalid input syntax".into()),

                    // numeric_value_out_of_range
                    (Some("22003"), _) => AppError::Validation("Numeric value out of range".into()),

                    _ => AppError::Db(e),
                }
            }
            _ => AppError::Db(e),
        }
    }
}