use serde::{Deserialize, Serialize};
use chrono::NaiveDate;

#[derive(Deserialize)]
pub struct CreateDeliveryRequest {
    pub delivery_date: NaiveDate,
    pub received_by: Option<i64>,
    pub delivery_note_number: String,
    pub items: Vec<NewDeliveryItem>,
}

#[derive(Deserialize)]
pub struct NewDeliveryItem {
    pub product_id: i64,
    pub unit_price: f64,
    pub batches: Vec<NewDeliveryBatch>,
}

#[derive(Deserialize)]
pub struct NewDeliveryBatch {
    pub batch_number: String,
    pub quantity: i32,
    pub expiry_date: NaiveDate,
}

#[derive(Serialize)]
pub struct DeliveryResponse {
    pub id: i64,
    pub delivery_date: NaiveDate,
    pub received_by: Option<i64>,
    pub delivery_note_number: String,
    pub items: Vec<DeliveryItemResponse>,
}

#[derive(Serialize)]
pub struct DeliveryItemResponse {
    pub id: i64,
    pub product_id: i64,
    pub quantity: i32,
    pub unit_price: f64,
    pub batches: Vec<DeliveryBatchResponse>,
}

#[derive(Serialize)]
pub struct DeliveryBatchResponse {
    pub id: i64,
    pub batch_number: String,
    pub quantity: i32,
    pub remaining_quantity: i32,
    pub expiry_date: NaiveDate,
}

#[derive(Serialize)]
pub struct DeliverySummary {
    pub id: i64,
    pub delivery_date: NaiveDate,
    pub delivery_note_number: String,
    pub received_by: Option<i64>,
    pub total_items: i64,
}

#[derive(Deserialize)]
pub struct UpdateDeliveryRequest {
    pub delivery_date: Option<NaiveDate>,
    pub received_by: Option<Option<i64>>, // Some(Some(id)) set, Some(None) clear, None ignore
    pub delivery_note_number: Option<String>,
}
