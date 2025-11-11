use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, NaiveDate};

#[derive(Deserialize)]
pub struct CreateTruckLoadRequest {
    pub truck_id: i64,
    pub load_date: NaiveDate,
    pub loaded_by: i64,
    pub notes: Option<String>,
    pub items: Vec<TruckLoadItemRequest>,
}

#[derive(Deserialize)]
pub struct TruckLoadItemRequest {
    pub batch_id: i64,
    pub quantity_loaded: i32,
}

#[derive(Deserialize)]
pub struct ReconcileTruckLoadRequest {
    pub returns: Vec<TruckLoadReturnItem>,
}

#[derive(Deserialize)]
pub struct TruckLoadReturnItem {
    pub batch_id: i64,
    pub quantity_returned: i32,
}

#[derive(Serialize)]
pub struct TruckLoadResponse {
    pub id: i64,
    pub truck_id: i64,
    pub truck_number: String,
    pub driver_username: Option<String>,
    pub load_date: NaiveDate,
    pub loaded_by: i64,
    pub loaded_by_username: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub items: Vec<TruckLoadItemResponse>,
    pub summary: TruckLoadSummary,
}

#[derive(Serialize)]
pub struct TruckLoadItemResponse {
    pub id: i64,
    pub batch_id: i64,
    pub batch_number: String,
    pub product_id: i64,
    pub product_name: String,
    pub expiry_date: NaiveDate,
    pub quantity_loaded: i32,
    pub quantity_sold: i32,
    pub quantity_returned: i32,
    pub quantity_lost_damaged: i32,
}

#[derive(Serialize)]
pub struct TruckLoadSummary {
    pub total_loaded: i32,
    pub total_sold: i32,
    pub total_returned: i32,
    pub total_lost_damaged: i32,
    pub product_lines: i32,
}

#[derive(Serialize)]
pub struct TruckLoadListItem {
    pub id: i64,
    pub truck_id: i64,
    pub truck_number: String,
    pub driver_username: Option<String>,
    pub load_date: NaiveDate,
    pub status: String,
    pub total_loaded: i32,
    pub total_sold: i32,
    pub total_returned: i32,
    pub total_lost_damaged: i32,
}
