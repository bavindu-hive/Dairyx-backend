use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

// ==================== Enums ====================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "stock_movement_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum StockMovementType {
    DeliveryIn,    // Stock received from CreamyLand delivery
    TruckLoadOut,  // Stock loaded onto truck
    SaleOut,       // Stock sold from truck to shop
    TruckReturnIn, // Stock returned from truck to batch
    Adjustment,    // Manual adjustment (damaged, expired, correction)
    ExpiredOut,    // Stock removed due to expiry
}

// ==================== Reconciliation DTOs ====================

#[derive(Debug, Deserialize)]
pub struct StartReconciliationRequest {
    pub reconciliation_date: NaiveDate,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyTruckReturnRequest {
    pub items_returned: Vec<TruckReturnItem>,
    pub items_discarded: Vec<DiscardedItem>,
    pub discrepancy_notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TruckReturnItem {
    pub product_id: i64,
    pub quantity: i32,
}

#[derive(Debug, Deserialize)]
pub struct DiscardedItem {
    pub product_id: i64,
    pub quantity: i32,
    pub reason: String, // "damaged", "expired", "wasted"
}

#[derive(Debug, Serialize)]
pub struct ReconciliationResponse {
    pub id: i64,
    pub reconciliation_date: NaiveDate,
    pub status: String,

    // Truck summary
    pub trucks_out: i32,
    pub trucks_verified: i32,

    // Stock summary
    pub total_items_loaded: f64,
    pub total_items_sold: f64,
    pub total_items_returned: f64,
    pub total_items_discarded: f64,

    // Financial summary
    pub total_sales_amount: f64,
    pub total_commission_earned: f64,
    pub total_allowance_allocated: f64,
    pub total_payments_collected: f64,
    pub pending_payments: f64,
    pub net_profit: f64,

    // Metadata
    pub started_by: Option<i64>,
    pub started_by_username: Option<String>,
    pub started_at: chrono::NaiveDateTime,
    pub finalized_by: Option<i64>,
    pub finalized_by_username: Option<String>,
    pub finalized_at: Option<chrono::NaiveDateTime>,
    pub notes: Option<String>,

    // Truck items
    pub truck_items: Vec<TruckVerificationItem>,
}

#[derive(Debug, Serialize)]
pub struct TruckVerificationItem {
    pub id: i64,
    pub truck_id: i64,
    pub truck_number: String,
    pub driver_id: i64,
    pub driver_username: String,
    pub truck_load_id: i64,

    // Stock verification
    pub items_loaded: f64,
    pub items_sold: f64,
    pub items_returned: f64,
    pub items_discarded: f64,

    // Status
    pub is_verified: bool,
    pub has_discrepancy: bool,
    pub discrepancy_notes: Option<String>,

    // Financial
    pub sales_amount: f64,
    pub commission_earned: f64,
    pub allowance_received: f64,
    pub payments_collected: f64,
    pub pending_payments: f64,

    pub verified_by: Option<i64>,
    pub verified_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Serialize)]
pub struct ReconciliationSummary {
    pub id: i64,
    pub reconciliation_date: NaiveDate,
    pub status: String,
    pub trucks_out: i32,
    pub trucks_verified: i32,
    pub net_profit: f64,
    pub profit_status: String, // "profit" or "loss"
    pub started_at: chrono::NaiveDateTime,
    pub finalized_at: Option<chrono::NaiveDateTime>,
}

// ==================== Stock Movement DTOs ====================

#[derive(Debug, Deserialize)]
pub struct CreateStockAdjustmentRequest {
    pub batch_id: i64,
    pub product_id: i64,
    pub quantity: f64,
    pub movement_type: StockMovementType, // Use enum instead of String
    pub reason: String,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StockMovementResponse {
    pub id: i32,
    pub batch_id: i32,
    pub product_id: i64,
    pub product_name: String,
    pub movement_type: StockMovementType,
    pub quantity: f64,
    pub reference_type: String,
    pub reference_id: i32,
    pub notes: Option<String>,
    pub created_by: Option<i64>,
    pub created_by_username: Option<String>,
    pub movement_date: NaiveDate,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct BatchMovementHistory {
    pub batch_id: i64,
    pub batch_number: String,
    pub product_id: i64,
    pub product_name: String,
    pub initial_quantity: i32,
    pub current_remaining: i32,
    pub movements: Vec<StockMovementDetail>,
}

#[derive(Debug, Serialize)]
pub struct StockMovementDetail {
    pub id: i32,
    pub movement_type: StockMovementType,
    pub quantity: f64,
    pub reference_type: String,
    pub reference_id: i32,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    pub movement_date: NaiveDate,
    pub running_balance: f64,
}

#[derive(Debug, Serialize)]
pub struct DailyStockSummary {
    pub movement_date: NaiveDate,
    pub product_summaries: Vec<ProductStockSummary>,
}

#[derive(Debug, Serialize)]
pub struct ProductStockSummary {
    pub product_id: i64,
    pub product_name: String,
    pub movements: Vec<MovementTypeSummary>,
}

#[derive(Debug, Serialize)]
pub struct MovementTypeSummary {
    pub movement_type: StockMovementType,
    pub transaction_count: i64,
    pub total_quantity: f64,
}
