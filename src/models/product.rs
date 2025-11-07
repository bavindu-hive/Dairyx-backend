use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, FromRow)]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub current_wholesale_price: f64,
    pub commission_per_unit: f64,
    pub created_at: Option<DateTime<Utc>>,
}