use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Deserialize)]
pub struct CreateShopRequest {
    pub name: String,
    pub location: Option<String>,
    pub contact_info: Option<String>,
    pub distance: Option<f64>,
}

#[derive(Deserialize)]
pub struct UpdateShopRequest {
    pub name: Option<String>,
    pub location: Option<String>,
    pub contact_info: Option<String>,
    pub distance: Option<f64>,
}

#[derive(Serialize)]
pub struct ShopResponse {
    pub id: i64,
    pub name: String,
    pub location: Option<String>,
    pub contact_info: Option<String>,
    pub distance: Option<f64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct ShopSummary {
    pub id: i64,
    pub name: String,
    pub location: Option<String>,
    pub distance: Option<f64>,
}
