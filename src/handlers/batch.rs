use axum::{extract::{State, Path, Query}, Json};
use serde::Deserialize;
use sqlx::Row;
use crate::state::AppState;
use crate::error::AppError;
use crate::dtos::batch::{BatchResponse, BatchListItem};

#[derive(Deserialize)]
pub struct BatchQueryParams {
    pub product_id: Option<i64>,
    pub status: Option<String>, // "available", "empty", "expired"
}

pub async fn list_batches(
    State(AppState { db_pool }): State<AppState>,
    Query(params): Query<BatchQueryParams>,
) -> Result<Json<Vec<BatchListItem>>, AppError> {
    let mut query = String::from(
        r#"SELECT 
            b.id, b.batch_number, b.product_id, p.name as product_name,
            b.quantity as initial_quantity, b.remaining_quantity, b.expiry_date,
            CASE 
                WHEN b.remaining_quantity = 0 THEN 'empty'
                WHEN b.expiry_date < CURRENT_DATE THEN 'expired'
                ELSE 'available'
            END as status
        FROM batches b
        JOIN products p ON b.product_id = p.id
        WHERE 1=1"#
    );

    if let Some(product_id) = params.product_id {
        query.push_str(&format!(" AND b.product_id = {}", product_id));
    }

    if let Some(status) = &params.status {
        match status.as_str() {
            "available" => query.push_str(" AND b.remaining_quantity > 0 AND b.expiry_date >= CURRENT_DATE"),
            "empty" => query.push_str(" AND b.remaining_quantity = 0"),
            "expired" => query.push_str(" AND b.expiry_date < CURRENT_DATE"),
            _ => return Err(AppError::validation("Invalid status. Use: available, empty, or expired")),
        }
    }

    query.push_str(" ORDER BY b.expiry_date ASC, b.created_at ASC");

    let rows = sqlx::query(&query).fetch_all(&db_pool).await?;

    let batches: Vec<BatchListItem> = rows.iter().map(|row| {
        use sqlx::Row;
        BatchListItem {
            id: row.get("id"),
            batch_number: row.get("batch_number"),
            product_id: row.get("product_id"),
            product_name: row.get("product_name"),
            initial_quantity: row.get("initial_quantity"),
            remaining_quantity: row.get("remaining_quantity"),
            expiry_date: row.get("expiry_date"),
            status: row.get("status"),
        }
    }).collect();

    Ok(Json(batches))
}

pub async fn get_batch(
    State(AppState { db_pool }): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<BatchResponse>, AppError> {
    let row = sqlx::query(
        r#"SELECT 
            b.id, b.batch_number, b.product_id, p.name as product_name,
            b.delivery_id, b.quantity as initial_quantity, 
            b.remaining_quantity, b.expiry_date, b.created_at
        FROM batches b
        JOIN products p ON b.product_id = p.id
        WHERE b.id = $1"#
    )
    .bind(id)
    .fetch_optional(&db_pool)
    .await?
    .ok_or_else(|| AppError::not_found("Batch not found"))?;

    Ok(Json(BatchResponse {
        id: row.get("id"),
        batch_number: row.get("batch_number"),
        product_id: row.get("product_id"),
        product_name: row.get("product_name"),
        delivery_id: row.get("delivery_id"),
        initial_quantity: row.get("initial_quantity"),
        remaining_quantity: row.get("remaining_quantity"),
        expiry_date: row.get("expiry_date"),
        created_at: row.get("created_at"),
    }))
}
