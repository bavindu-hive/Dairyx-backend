use serde::Serialize;
use chrono::{NaiveDate, DateTime, Utc};

#[derive(Serialize)]
pub struct BatchResponse {
    pub id: i64,
    pub batch_number: String,
    pub product_id: i64,
    pub product_name: String,
    pub delivery_id: i64,
    pub initial_quantity: i32,
    pub remaining_quantity: i32,
    pub expiry_date: NaiveDate,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct BatchListItem {
    pub id: i64,
    pub batch_number: String,
    pub product_id: i64,
    pub product_name: String,
    pub initial_quantity: i32,
    pub remaining_quantity: i32,
    pub expiry_date: NaiveDate,
    pub status: String, // "available", "empty", "expired"
}
