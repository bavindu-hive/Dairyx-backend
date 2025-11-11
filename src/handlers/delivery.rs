use axum::{extract::State, Json};
use axum::http::StatusCode;
use chrono::NaiveDate;
use crate::state::AppState;
use crate::error::AppError;
use crate::dtos::delivery::{
    CreateDeliveryRequest, DeliveryResponse, DeliveryItemResponse, DeliveryBatchResponse,
    DeliverySummary, UpdateDeliveryRequest, NewDeliveryItem
};
use crate::middleware::auth::AuthContext;
use axum::extract::Extension;

pub async fn create_delivery(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateDeliveryRequest>,
) -> Result<(StatusCode, Json<DeliveryResponse>), AppError> {
    if auth.role != "manager" { return Err(AppError::forbidden("Only managers can create deliveries")); }
    if req.items.is_empty() { return Err(AppError::validation("Delivery must have at least one item")); }

    for item in &req.items {
        if item.unit_price < 0.0 {
            return Err(AppError::validation("unit_price must be greater than or equal to 0"));
        }
        if item.batches.is_empty() { return Err(AppError::validation("Each item must have at least one batch")); }
        for b in &item.batches { if b.quantity <= 0 { return Err(AppError::validation("Batch quantity must be > 0")); } }
    }

    let mut tx = db_pool.begin().await?;

    let delivery = sqlx::query!(
        r#"INSERT INTO deliveries (delivery_date, received_by, delivery_note_number) VALUES ($1,$2,$3)
        RETURNING id, delivery_date, received_by, delivery_note_number"#,
        req.delivery_date,
        req.received_by,
        req.delivery_note_number
    ).fetch_one(&mut *tx).await?;

    let mut items_out: Vec<DeliveryItemResponse> = Vec::with_capacity(req.items.len());

    for item in &req.items {
        let total_qty: i32 = item.batches.iter().map(|b| b.quantity).sum();
        let item_row = sqlx::query!(
            r#"INSERT INTO delivery_items (delivery_id, product_id, quantity, unit_price)
            VALUES ($1,$2,$3,$4::FLOAT8)
            RETURNING id, product_id, quantity, unit_price::FLOAT8 as "unit_price!""#,
            delivery.id,
            item.product_id,
            total_qty,
            item.unit_price
        ).fetch_one(&mut *tx).await.map_err(|e| {
            if let Some(db) = e.as_database_error() {
                if db.code().as_deref() == Some("23503") { return AppError::validation("Invalid product_id or received_by"); }
                if db.code().as_deref() == Some("23505") { return AppError::conflict("Product already exists in delivery"); }
            }
            AppError::db(e)
        })?;

        let mut batches_out = Vec::with_capacity(item.batches.len());
        for b in &item.batches {
            // Try to find existing batch by (product_id, batch_number)
            let existing = sqlx::query!(
                r#"SELECT id, expiry_date, quantity, remaining_quantity FROM batches WHERE product_id = $1 AND batch_number = $2"#,
                item.product_id,
                b.batch_number
            ).fetch_optional(&mut *tx).await?;

            let (batch_id, remaining_qty) = match existing {
                Some(ex) => {
                    // Validate expiry date matches
                    if ex.expiry_date != b.expiry_date {
                        return Err(AppError::validation(format!("Batch {} already exists with different expiry date", b.batch_number)));
                    }
                    // Update batch quantities
                    let new_qty = ex.quantity + b.quantity;
                    let new_remaining = ex.remaining_quantity + b.quantity;
                    sqlx::query!(
                        r#"UPDATE batches SET quantity = $1, remaining_quantity = $2 WHERE id = $3"#,
                        new_qty,
                        new_remaining,
                        ex.id
                    ).execute(&mut *tx).await?;
                    (ex.id, new_remaining)
                }
                None => {
                    // Create new batch
                    let new_batch = sqlx::query!(
                        r#"INSERT INTO batches (product_id, delivery_id, delivery_item_id, batch_number, quantity, remaining_quantity, expiry_date)
                        VALUES ($1,$2,$3,$4,$5,$5,$6)
                        RETURNING id, remaining_quantity"#,
                        item.product_id,
                        delivery.id,
                        item_row.id,
                        b.batch_number,
                        b.quantity,
                        b.expiry_date
                    ).fetch_one(&mut *tx).await.map_err(|e| {
                        if let Some(db) = e.as_database_error() {
                            if db.code().as_deref() == Some("23505") { return AppError::conflict("Batch number exists for product"); }
                            if db.code().as_deref() == Some("23503") { return AppError::validation("Invalid delivery/product reference"); }
                        }
                        AppError::db(e)
                    })?;
                    (new_batch.id, new_batch.remaining_quantity)
                }
            };

            // Insert batch_receipts linking this batch to this delivery_item
            sqlx::query!(
                r#"INSERT INTO batch_receipts (batch_id, delivery_id, delivery_item_id, quantity)
                VALUES ($1,$2,$3,$4)"#,
                batch_id,
                delivery.id,
                item_row.id,
                b.quantity
            ).execute(&mut *tx).await?;

            batches_out.push(DeliveryBatchResponse {
                id: batch_id,
                batch_number: b.batch_number.clone(),
                quantity: b.quantity,
                remaining_quantity: remaining_qty,
                expiry_date: b.expiry_date,
            });
        }

        items_out.push(DeliveryItemResponse { id: item_row.id, product_id: item_row.product_id, quantity: item_row.quantity, unit_price: item_row.unit_price, batches: batches_out });
    }

    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(DeliveryResponse { id: delivery.id, delivery_date: delivery.delivery_date, received_by: delivery.received_by, delivery_note_number: delivery.delivery_note_number, items: items_out })))
}

pub async fn get_delivery(
    State(AppState { db_pool }): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<DeliveryResponse>, AppError> {
    let d = sqlx::query!(
        r#"SELECT id, delivery_date, received_by, delivery_note_number FROM deliveries WHERE id = $1"#,
        id
    ).fetch_optional(&db_pool).await?.ok_or_else(|| AppError::not_found("Delivery not found"))?;

    let items = sqlx::query!(
        r#"SELECT id, product_id, quantity, unit_price::FLOAT8 as "unit_price!" FROM delivery_items WHERE delivery_id = $1 ORDER BY id"#,
        id
    ).fetch_all(&db_pool).await?;

    let mut items_out = Vec::with_capacity(items.len());
    for it in items {
        // Fetch batch receipts for this delivery_item to get per-delivery quantities
        let receipt_batches = sqlx::query!(
            r#"SELECT br.batch_id, br.quantity as receipt_qty, b.batch_number, b.remaining_quantity, b.expiry_date
               FROM batch_receipts br
               JOIN batches b ON b.id = br.batch_id
               WHERE br.delivery_item_id = $1
               ORDER BY b.expiry_date ASC, b.id ASC"#,
            it.id
        ).fetch_all(&db_pool).await?;
        
        items_out.push(DeliveryItemResponse {
            id: it.id,
            product_id: it.product_id,
            quantity: it.quantity,
            unit_price: it.unit_price,
            batches: receipt_batches.into_iter().map(|rb| DeliveryBatchResponse {
                id: rb.batch_id,
                batch_number: rb.batch_number,
                quantity: rb.receipt_qty,
                remaining_quantity: rb.remaining_quantity,
                expiry_date: rb.expiry_date,
            }).collect(),
        });
    }

    Ok(Json(DeliveryResponse { id: d.id, delivery_date: d.delivery_date, received_by: d.received_by, delivery_note_number: d.delivery_note_number, items: items_out }))
}

pub async fn list_deliveries(
    State(AppState { db_pool }): State<AppState>,
) -> Result<Json<Vec<DeliverySummary>>, AppError> {
    let rows = sqlx::query!(
        r#"SELECT d.id, d.delivery_date, d.delivery_note_number, d.received_by, COUNT(di.id)::BIGINT as total_items
            FROM deliveries d LEFT JOIN delivery_items di ON di.delivery_id = d.id
            GROUP BY d.id, d.delivery_date, d.delivery_note_number, d.received_by
            ORDER BY d.delivery_date DESC, d.id DESC"#
    ).fetch_all(&db_pool).await?;

    Ok(Json(rows.into_iter().map(|r| DeliverySummary { id: r.id, delivery_date: r.delivery_date, delivery_note_number: r.delivery_note_number, received_by: r.received_by, total_items: r.total_items.unwrap_or(0) }).collect()))
}

pub async fn update_delivery(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<UpdateDeliveryRequest>,
) -> Result<Json<DeliveryResponse>, AppError> {
    if auth.role != "manager" { return Err(AppError::forbidden("Only managers can update deliveries")); }
            let row = sqlx::query!(
                    r#"UPDATE deliveries SET delivery_date = COALESCE($2, delivery_date),
                        received_by = COALESCE($3::BIGINT, received_by),
                        delivery_note_number = COALESCE($4, delivery_note_number)
                        WHERE id = $1
                        RETURNING id, delivery_date, received_by, delivery_note_number"#,
                id,
                req.delivery_date,
                req.received_by.flatten(),
                req.delivery_note_number
        ).fetch_optional(&db_pool).await?.ok_or_else(|| AppError::not_found("Delivery not found"))?;

    let items = sqlx::query!(
        r#"SELECT id, product_id, quantity, unit_price::FLOAT8 as "unit_price!" FROM delivery_items WHERE delivery_id = $1 ORDER BY id"#,
        id
    ).fetch_all(&db_pool).await?;

    let mut items_out = Vec::with_capacity(items.len());
    for it in items {
        // Fetch batch receipts for this delivery_item to get per-delivery quantities
        let receipt_batches = sqlx::query!(
            r#"SELECT br.batch_id, br.quantity as receipt_qty, b.batch_number, b.remaining_quantity, b.expiry_date
               FROM batch_receipts br
               JOIN batches b ON b.id = br.batch_id
               WHERE br.delivery_item_id = $1
               ORDER BY b.expiry_date ASC, b.id ASC"#,
            it.id
        ).fetch_all(&db_pool).await?;
        
        items_out.push(DeliveryItemResponse {
            id: it.id,
            product_id: it.product_id,
            quantity: it.quantity,
            unit_price: it.unit_price,
            batches: receipt_batches.into_iter().map(|rb| DeliveryBatchResponse {
                id: rb.batch_id,
                batch_number: rb.batch_number,
                quantity: rb.receipt_qty,
                remaining_quantity: rb.remaining_quantity,
                expiry_date: rb.expiry_date,
            }).collect(),
        });
    }

    Ok(Json(DeliveryResponse { id: row.id, delivery_date: row.delivery_date, received_by: row.received_by, delivery_note_number: row.delivery_note_number, items: items_out }))
}

pub async fn delete_delivery(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<StatusCode, AppError> {
    if auth.role != "manager" { return Err(AppError::forbidden("Only managers can delete deliveries")); }
    let mut tx = db_pool.begin().await?;
    
    // Check if any batch linked to this delivery has sales
    let locked = sqlx::query_scalar!(
        r#"SELECT EXISTS (
            SELECT 1 FROM sale_items si
            JOIN batches b ON b.id = si.batch_id
            JOIN batch_receipts br ON br.batch_id = b.id
            WHERE br.delivery_id = $1
        ) as "exists!""#,
        id
    ).fetch_one(&mut *tx).await?;
    if locked { return Err(AppError::conflict("Cannot delete delivery with sold batches")); }

    // Get all batch_receipts for this delivery
    let receipts = sqlx::query!(
        r#"SELECT batch_id, quantity FROM batch_receipts WHERE delivery_id = $1"#,
        id
    ).fetch_all(&mut *tx).await?;

    // Reduce batch quantities for each receipt, delete batch if quantity becomes 0
    for r in receipts {
        let updated = sqlx::query!(
            r#"UPDATE batches SET quantity = quantity - $1, remaining_quantity = remaining_quantity - $1
               WHERE id = $2
               RETURNING quantity"#,
            r.quantity,
            r.batch_id
        ).fetch_one(&mut *tx).await?;

        // If batch is now empty, delete it
        if updated.quantity <= 0 {
            sqlx::query!("DELETE FROM batches WHERE id = $1", r.batch_id).execute(&mut *tx).await?;
        }
    }

    // Delete batch_receipts
    sqlx::query!("DELETE FROM batch_receipts WHERE delivery_id = $1", id).execute(&mut *tx).await?;
    
    // Delete delivery_items
    sqlx::query!("DELETE FROM delivery_items WHERE delivery_id = $1", id).execute(&mut *tx).await?;
    
    // Delete delivery
    let res = sqlx::query!("DELETE FROM deliveries WHERE id = $1", id).execute(&mut *tx).await?;
    
    tx.commit().await?;
    if res.rows_affected() == 0 { return Err(AppError::not_found("Delivery not found")); }
    Ok(StatusCode::NO_CONTENT)
}
