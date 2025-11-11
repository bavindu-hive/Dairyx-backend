use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Deserialize)]
pub struct CreateTruckRequest {
    pub truck_number: String,
    pub driver_id: Option<i64>,
}

#[derive(Deserialize)]
pub struct UpdateTruckRequest {
    pub truck_number: Option<String>,
    pub driver_id: Option<Option<i64>>, // Some(Some(id)) set, Some(None) clear, None ignore
    pub is_active: Option<bool>,
}

#[derive(Serialize)]
pub struct TruckResponse {
    pub id: i64,
    pub truck_number: String,
    pub driver_id: Option<i64>,
    pub driver_username: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct TruckSummary {
    pub id: i64,
    pub truck_number: String,
    pub driver_username: Option<String>,
    pub is_active: bool,
}
