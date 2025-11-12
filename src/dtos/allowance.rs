use serde::{Deserialize, Serialize};
use chrono::{DateTime, NaiveDate, Utc};

// Request DTOs

#[derive(Deserialize)]
pub struct CreateTransportAllowanceRequest {
    pub allowance_date: NaiveDate,
    pub total_allowance: f64,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct AllocateToTrucksRequest {
    pub allocations: Vec<TruckAllocationRequest>,
}

#[derive(Deserialize)]
pub struct TruckAllocationRequest {
    pub truck_id: i64,
    pub amount: f64,
    pub distance_covered: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateTruckAllocationRequest {
    pub amount: f64,
    pub distance_covered: Option<f64>,
    pub notes: Option<String>,
}

// Response DTOs

#[derive(Serialize)]
pub struct TransportAllowanceResponse {
    pub id: i64,
    pub allowance_date: NaiveDate,
    pub total_allowance: f64,
    pub allocated_amount: f64,
    pub remaining_amount: f64,
    pub status: String,
    pub notes: Option<String>,
    pub created_by_username: String,
    pub truck_allocations: Vec<TruckAllocationResponse>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct TruckAllocationResponse {
    pub id: i64,
    pub truck_id: i64,
    pub truck_number: String,
    pub driver_username: Option<String>,
    pub max_limit: f64,
    pub amount: f64,
    pub distance_covered: Option<f64>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct AllowanceSummary {
    pub id: i64,
    pub allowance_date: NaiveDate,
    pub total_allowance: f64,
    pub allocated_amount: f64,
    pub remaining_amount: f64,
    pub status: String,
    pub truck_count: i32,
    pub created_by_username: String,
}
