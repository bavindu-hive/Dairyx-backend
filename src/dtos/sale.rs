use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, NaiveDate};

#[derive(Deserialize)]
pub struct CreateSaleRequest {
    pub shop_id: i64,
    pub truck_load_id: i64,
    pub sale_date: NaiveDate,
    pub amount_paid: Option<f64>,
    pub items: Vec<SaleItemRequest>,
}

#[derive(Deserialize)]
pub struct SaleItemRequest {
    pub product_id: i64,
    pub quantity: i32,
    pub unit_price: Option<f64>, // Optional - uses current_wholesale_price if not provided
}

#[derive(Deserialize)]
pub struct UpdatePaymentRequest {
    pub additional_payment: f64,
}

#[derive(Serialize)]
pub struct SaleResponse {
    pub id: i64,
    pub shop_id: i64,
    pub shop_name: String,
    pub truck_id: i64,
    pub truck_number: String,
    pub driver_id: i64,
    pub driver_username: String,
    pub truck_load_id: i64,
    pub total_amount: f64,
    pub amount_paid: f64,
    pub payment_status: String,
    pub sale_date: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub items: Vec<SaleItemResponse>,
    pub summary: SaleSummary,
}

#[derive(Serialize)]
pub struct SaleItemResponse {
    pub id: i64,
    pub product_id: i64,
    pub product_name: String,
    pub batch_id: i64,
    pub batch_number: String,
    pub quantity: i32,
    pub unit_price: f64,
    pub commission_earned: f64,
    pub line_total: f64,
}

#[derive(Serialize)]
pub struct SaleSummary {
    pub total_items: i32,
    pub total_commission: f64,
    pub balance_due: f64,
}

#[derive(Serialize)]
pub struct SaleListItem {
    pub id: i64,
    pub shop_name: String,
    pub truck_number: String,
    pub driver_username: String,
    pub total_amount: f64,
    pub amount_paid: f64,
    pub payment_status: String,
    pub sale_date: NaiveDate,
    pub total_items: i32,
}
