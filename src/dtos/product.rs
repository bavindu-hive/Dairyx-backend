// src/dtos/product.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub current_wholesale_price: f64,
    pub commission_per_unit: f64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub current_wholesale_price: Option<f64>,
    pub commission_per_unit: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ProductResponse {
    pub id: i64,
    pub name: String,
    pub current_wholesale_price: f64,
    pub commission_per_unit: f64,
    pub created_at: Option<String>,
}

// Convert from Model to Response DTO
impl From<crate::models::product::Product> for ProductResponse {
    fn from(product: crate::models::product::Product) -> Self {
        Self {
            id: product.id,
            name: product.name,
            current_wholesale_price: product.current_wholesale_price,
            commission_per_unit: product.commission_per_unit,
            created_at: product.created_at.map(|dt| dt.to_rfc3339()),
        }
    }
}