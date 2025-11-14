use crate::dtos::truck_load::{
    CreateTruckLoadRequest, ReconcileTruckLoadRequest, TruckLoadItemResponse, TruckLoadListItem,
    TruckLoadResponse, TruckLoadSummary,
};
use crate::error::AppError;
use crate::middleware::auth::AuthContext;
use crate::state::AppState;
use axum::http::StatusCode;
use axum::{extract::State, Extension, Json};
use sqlx::PgPool;

pub async fn create_truck_load(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateTruckLoadRequest>,
) -> Result<(StatusCode, Json<TruckLoadResponse>), AppError> {
    if auth.role != "manager" {
        return Err(AppError::forbidden("Only managers can create truck loads"));
    }

    if req.items.is_empty() {
        return Err(AppError::validation(
            "Truck load must contain at least one item",
        ));
    }

    // Verify truck exists and is active
    let truck = sqlx::query!(
        r#"SELECT t.id, t.truck_number, t.is_active, u.username as "driver_username?"
        FROM trucks t
        LEFT JOIN users u ON t.driver_id = u.id
        WHERE t.id = $1"#,
        req.truck_id
    )
    .fetch_optional(&db_pool)
    .await?
    .ok_or_else(|| AppError::not_found("Truck not found"))?;

    if !truck.is_active {
        return Err(AppError::validation("Truck is not active"));
    }

    // Start transaction
    let mut tx = db_pool.begin().await?;

    // Create truck load
    let truck_load = sqlx::query!(
        r#"INSERT INTO truck_loads (truck_id, load_date, loaded_by, notes)
        VALUES ($1, $2, $3, $4)
        RETURNING id, truck_id, load_date, loaded_by, status, notes, created_at"#,
        req.truck_id,
        req.load_date,
        req.loaded_by,
        req.notes
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        if let Some(db) = e.as_database_error() {
            if db.code().as_deref() == Some("23505") {
                return AppError::conflict(
                    "A truck load already exists for this truck on this date",
                );
            }
        }
        AppError::db(e)
    })?;

    // Validate and insert items
    let mut items = Vec::new();
    for item in &req.items {
        // Validate that exactly one of batch_id or product_id is provided
        match (item.batch_id, item.product_id) {
            (None, None) => {
                return Err(AppError::validation(
                    "Each item must have either batch_id or product_id",
                ));
            }
            (Some(_), Some(_)) => {
                return Err(AppError::validation(
                    "Each item cannot have both batch_id and product_id",
                ));
            }
            (Some(batch_id), None) => {
                // Manual batch selection (existing logic)
                let loaded_items = load_specific_batch(
                    &mut tx,
                    truck_load.id as i64,
                    batch_id,
                    item.quantity_loaded,
                )
                .await?;
                items.extend(loaded_items);
            }
            (None, Some(product_id)) => {
                // Auto FIFO batch selection
                let loaded_items = load_product_fifo(
                    &mut tx,
                    truck_load.id as i64,
                    product_id,
                    item.quantity_loaded,
                )
                .await?;
                items.extend(loaded_items);
            }
        }
    }

    // Commit transaction
    tx.commit().await?;

    // Get loaded_by username
    let loaded_by_username = sqlx::query_scalar!(
        r#"SELECT username FROM users WHERE id = $1"#,
        truck_load.loaded_by
    )
    .fetch_optional(&db_pool)
    .await?;

    // Calculate summary
    let total_loaded: i32 = items.iter().map(|i| i.quantity_loaded).sum();
    let total_sold: i32 = items.iter().map(|i| i.quantity_sold).sum();
    let total_returned: i32 = items.iter().map(|i| i.quantity_returned).sum();
    let total_lost_damaged = total_loaded - total_sold - total_returned;

    Ok((
        StatusCode::CREATED,
        Json(TruckLoadResponse {
            id: truck_load.id,
            truck_id: truck_load.truck_id,
            truck_number: truck.truck_number,
            driver_username: truck.driver_username,
            load_date: truck_load.load_date,
            loaded_by: truck_load.loaded_by.unwrap(),
            loaded_by_username,
            status: truck_load.status,
            notes: truck_load.notes,
            created_at: truck_load.created_at.unwrap(),
            items,
            summary: TruckLoadSummary {
                total_loaded,
                total_sold,
                total_returned,
                total_lost_damaged,
                product_lines: req.items.len() as i32,
            },
        }),
    ))
}

pub async fn get_truck_load(
    State(AppState { db_pool }): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<TruckLoadResponse>, AppError> {
    fetch_truck_load_by_id(&db_pool, id).await.map(Json)
}

pub async fn list_truck_loads(
    State(AppState { db_pool }): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<TruckLoadListItem>>, AppError> {
    let truck_id = params.get("truck_id").and_then(|s| s.parse::<i64>().ok());
    let load_date = params
        .get("load_date")
        .and_then(|s| s.parse::<chrono::NaiveDate>().ok());
    let status = params.get("status");

    let mut query_str = String::from(
        r#"SELECT 
            tl.id, tl.truck_id, tl.load_date, tl.status,
            t.truck_number, u.username as driver_username,
            COALESCE(SUM(tli.quantity_loaded), 0)::INT as total_loaded,
            COALESCE(SUM(tli.quantity_sold), 0)::INT as total_sold,
            COALESCE(SUM(tli.quantity_returned), 0)::INT as total_returned
        FROM truck_loads tl
        JOIN trucks t ON tl.truck_id = t.id
        LEFT JOIN users u ON t.driver_id = u.id
        LEFT JOIN truck_load_items tli ON tl.id = tli.truck_load_id
        WHERE 1=1"#,
    );

    if truck_id.is_some() {
        query_str.push_str(" AND tl.truck_id = $1");
    }
    if load_date.is_some() {
        let param_num = if truck_id.is_some() { 2 } else { 1 };
        query_str.push_str(&format!(" AND tl.load_date = ${}", param_num));
    }
    if status.is_some() {
        let param_num = if truck_id.is_some() && load_date.is_some() {
            3
        } else if truck_id.is_some() || load_date.is_some() {
            2
        } else {
            1
        };
        query_str.push_str(&format!(" AND tl.status = ${}", param_num));
    }

    query_str.push_str(" GROUP BY tl.id, tl.truck_id, tl.load_date, tl.status, t.truck_number, u.username ORDER BY tl.load_date DESC, tl.id DESC");

    let mut query = sqlx::query_as::<
        _,
        (
            i64,
            i64,
            chrono::NaiveDate,
            String,
            String,
            Option<String>,
            i32,
            i32,
            i32,
        ),
    >(&query_str);

    if let Some(tid) = truck_id {
        query = query.bind(tid);
    }
    if let Some(date) = load_date {
        query = query.bind(date);
    }
    if let Some(st) = status {
        query = query.bind(st);
    }

    let loads = query.fetch_all(&db_pool).await?;

    Ok(Json(
        loads
            .into_iter()
            .map(
                |(
                    id,
                    truck_id,
                    load_date,
                    status,
                    truck_number,
                    driver_username,
                    total_loaded,
                    total_sold,
                    total_returned,
                )| {
                    TruckLoadListItem {
                        id,
                        truck_id,
                        truck_number,
                        driver_username,
                        load_date,
                        status,
                        total_loaded,
                        total_sold,
                        total_returned,
                        total_lost_damaged: total_loaded - total_sold - total_returned,
                    }
                },
            )
            .collect(),
    ))
}

pub async fn reconcile_truck_load(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<ReconcileTruckLoadRequest>,
) -> Result<Json<TruckLoadResponse>, AppError> {
    if auth.role != "manager" {
        return Err(AppError::forbidden(
            "Only managers can reconcile truck loads",
        ));
    }

    // Start transaction
    let mut tx = db_pool.begin().await?;

    // Verify truck load exists and is not already reconciled
    let truck_load = sqlx::query!(r#"SELECT id, status FROM truck_loads WHERE id = $1"#, id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::not_found("Truck load not found"))?;

    if truck_load.status == "reconciled" {
        return Err(AppError::conflict("Truck load is already reconciled"));
    }

    // Update return quantities
    for return_item in &req.returns {
        let result = sqlx::query!(
            r#"UPDATE truck_load_items
            SET quantity_returned = $2
            WHERE truck_load_id = $1 AND batch_id = $3
            RETURNING quantity_loaded, quantity_sold, quantity_returned"#,
            id,
            return_item.quantity_returned,
            return_item.batch_id
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            AppError::not_found(&format!(
                "Batch {} not found in this truck load",
                return_item.batch_id
            ))
        })?;

        // Validate returned quantity
        if result.quantity_sold + result.quantity_returned > result.quantity_loaded {
            return Err(AppError::validation(&format!(
                "Batch {}: Total sold ({}) + returned ({}) cannot exceed loaded quantity ({})",
                return_item.batch_id,
                result.quantity_sold,
                result.quantity_returned,
                result.quantity_loaded
            )));
        }

        // Restore returned quantity back to batch remaining_quantity
        sqlx::query!(
            r#"UPDATE batches 
            SET remaining_quantity = remaining_quantity + $2
            WHERE id = $1"#,
            return_item.batch_id,
            return_item.quantity_returned
        )
        .execute(&mut *tx)
        .await?;
    }

    // Update truck load status to reconciled
    sqlx::query!(
        r#"UPDATE truck_loads SET status = 'reconciled' WHERE id = $1"#,
        id
    )
    .execute(&mut *tx)
    .await?;

    // Commit transaction
    tx.commit().await?;

    // Fetch and return updated truck load
    fetch_truck_load_by_id(&db_pool, id).await.map(Json)
}

pub async fn delete_truck_load(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<StatusCode, AppError> {
    if auth.role != "manager" {
        return Err(AppError::forbidden("Only managers can delete truck loads"));
    }

    // Start transaction
    let mut tx = db_pool.begin().await?;

    // Check if truck load has any sales
    let has_sales = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM sales WHERE truck_load_id = $1) as "exists!""#,
        id
    )
    .fetch_one(&mut *tx)
    .await?;

    if has_sales {
        return Err(AppError::conflict(
            "Cannot delete truck load with existing sales",
        ));
    }

    // Get all items to restore their quantities
    let items = sqlx::query!(
        r#"SELECT batch_id, quantity_loaded, quantity_returned
        FROM truck_load_items
        WHERE truck_load_id = $1"#,
        id
    )
    .fetch_all(&mut *tx)
    .await?;

    // Restore quantities for items not returned
    for item in items {
        let quantity_to_restore = item.quantity_loaded - item.quantity_returned;
        if quantity_to_restore > 0 {
            sqlx::query!(
                r#"UPDATE batches 
                SET remaining_quantity = remaining_quantity + $2
                WHERE id = $1"#,
                item.batch_id,
                quantity_to_restore
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    // Delete truck load (cascade will delete items)
    let result = sqlx::query!("DELETE FROM truck_loads WHERE id = $1", id)
        .execute(&mut *tx)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("Truck load not found"));
    }

    // Commit transaction
    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

// Helper function to fetch full truck load details
async fn fetch_truck_load_by_id(db_pool: &PgPool, id: i64) -> Result<TruckLoadResponse, AppError> {
    // Fetch truck load header
    let truck_load = sqlx::query!(
        r#"SELECT 
            tl.id, tl.truck_id, tl.load_date, tl.loaded_by, tl.status, tl.notes, tl.created_at,
            t.truck_number,
            u1.username as "driver_username?",
            u2.username as "loaded_by_username?"
        FROM truck_loads tl
        JOIN trucks t ON tl.truck_id = t.id
        LEFT JOIN users u1 ON t.driver_id = u1.id
        LEFT JOIN users u2 ON tl.loaded_by = u2.id
        WHERE tl.id = $1"#,
        id
    )
    .fetch_optional(db_pool)
    .await?
    .ok_or_else(|| AppError::not_found("Truck load not found"))?;

    // Fetch truck load items
    let items_data = sqlx::query!(
        r#"SELECT 
            tli.id, tli.batch_id, tli.quantity_loaded, tli.quantity_sold, tli.quantity_returned,
            b.batch_number, b.product_id, b.expiry_date,
            p.name as product_name
        FROM truck_load_items tli
        JOIN batches b ON tli.batch_id = b.id
        JOIN products p ON b.product_id = p.id
        WHERE tli.truck_load_id = $1
        ORDER BY p.name, b.expiry_date"#,
        id
    )
    .fetch_all(db_pool)
    .await?;

    let items: Vec<TruckLoadItemResponse> = items_data
        .into_iter()
        .map(|item| {
            // Only calculate lost/damaged if truck is reconciled
            // For loaded status, we don't know yet what will be lost
            let quantity_lost_damaged = if truck_load.status == "reconciled" {
                item.quantity_loaded - item.quantity_sold - item.quantity_returned
            } else {
                0 // Truck is still out, we don't know losses yet
            };

            TruckLoadItemResponse {
                id: item.id,
                batch_id: item.batch_id,
                batch_number: item.batch_number,
                product_id: item.product_id,
                product_name: item.product_name,
                expiry_date: item.expiry_date,
                quantity_loaded: item.quantity_loaded,
                quantity_sold: item.quantity_sold,
                quantity_returned: item.quantity_returned,
                quantity_lost_damaged,
            }
        })
        .collect();

    // Calculate summary
    let total_loaded: i32 = items.iter().map(|i| i.quantity_loaded).sum();
    let total_sold: i32 = items.iter().map(|i| i.quantity_sold).sum();
    let total_returned: i32 = items.iter().map(|i| i.quantity_returned).sum();
    // Only calculate total lost/damaged if reconciled
    let total_lost_damaged = if truck_load.status == "reconciled" {
        total_loaded - total_sold - total_returned
    } else {
        0
    };
    let product_lines = items.len() as i32;

    Ok(TruckLoadResponse {
        id: truck_load.id,
        truck_id: truck_load.truck_id,
        truck_number: truck_load.truck_number,
        driver_username: truck_load.driver_username,
        load_date: truck_load.load_date,
        loaded_by: truck_load.loaded_by.unwrap(),
        loaded_by_username: truck_load.loaded_by_username,
        status: truck_load.status,
        notes: truck_load.notes,
        created_at: truck_load.created_at.unwrap(),
        items,
        summary: TruckLoadSummary {
            total_loaded,
            total_sold,
            total_returned,
            total_lost_damaged,
            product_lines,
        },
    })
}

// ==================== Helper Functions ====================

/// Load a specific batch (manual selection)
async fn load_specific_batch(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    truck_load_id: i64,
    batch_id: i64,
    quantity_loaded: i32,
) -> Result<Vec<TruckLoadItemResponse>, AppError> {
    // Verify batch exists and has enough quantity
    let batch = sqlx::query!(
        r#"SELECT b.id, b.product_id, b.batch_number, b.remaining_quantity, b.expiry_date, p.name as product_name
        FROM batches b
        JOIN products p ON b.product_id = p.id
        WHERE b.id = $1"#,
        batch_id
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| AppError::not_found(&format!("Batch {} not found", batch_id)))?;

    if batch.remaining_quantity < quantity_loaded {
        return Err(AppError::validation(&format!(
            "Batch {} only has {} units remaining, cannot load {}",
            batch.batch_number, batch.remaining_quantity, quantity_loaded
        )));
    }

    // Insert truck load item
    let load_item = sqlx::query!(
        r#"INSERT INTO truck_load_items (truck_load_id, batch_id, quantity_loaded)
        VALUES ($1, $2, $3)
        RETURNING id, truck_load_id, batch_id, quantity_loaded, quantity_sold, quantity_returned, created_at"#,
        truck_load_id as i32,
        batch_id,
        quantity_loaded
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| {
        if let Some(db) = e.as_database_error() {
            if db.code().as_deref() == Some("23505") {
                return AppError::conflict(&format!("Batch {} already added to this truck load", batch.batch_number));
            }
        }
        AppError::db(e)
    })?;

    // Deduct loaded quantity from batch remaining_quantity
    sqlx::query!(
        r#"UPDATE batches 
        SET remaining_quantity = remaining_quantity - $2
        WHERE id = $1"#,
        batch_id,
        quantity_loaded
    )
    .execute(&mut **tx)
    .await?;

    // Truck is just being loaded, no losses yet
    Ok(vec![TruckLoadItemResponse {
        id: load_item.id,
        batch_id: batch.id,
        batch_number: batch.batch_number,
        product_id: batch.product_id,
        product_name: batch.product_name,
        expiry_date: batch.expiry_date,
        quantity_loaded: load_item.quantity_loaded,
        quantity_sold: load_item.quantity_sold,
        quantity_returned: load_item.quantity_returned,
        quantity_lost_damaged: 0, // Status is 'loaded', losses not determined yet
    }])
}

/// Load product using FIFO (First In First Out by expiry date)
async fn load_product_fifo(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    truck_load_id: i64,
    product_id: i64,
    total_quantity_needed: i32,
) -> Result<Vec<TruckLoadItemResponse>, AppError> {
    // Get available batches for this product, ordered by expiry date (FIFO)
    let batches = sqlx::query!(
        r#"SELECT b.id, b.batch_number, b.remaining_quantity, b.expiry_date, p.name as product_name
        FROM batches b
        JOIN products p ON b.product_id = p.id
        WHERE b.product_id = $1 AND b.remaining_quantity > 0
        ORDER BY b.expiry_date ASC, b.created_at ASC"#,
        product_id
    )
    .fetch_all(&mut **tx)
    .await?;

    if batches.is_empty() {
        return Err(AppError::not_found(&format!(
            "No available batches found for product {}",
            product_id
        )));
    }

    // Calculate total available quantity
    let total_available: i32 = batches.iter().map(|b| b.remaining_quantity).sum();
    if total_available < total_quantity_needed {
        return Err(AppError::validation(&format!(
            "Insufficient stock for product {}. Available: {}, Requested: {}",
            product_id, total_available, total_quantity_needed
        )));
    }

    // Allocate quantity across batches using FIFO
    let mut remaining_to_load = total_quantity_needed;
    let mut loaded_items = Vec::new();

    for batch in batches {
        if remaining_to_load == 0 {
            break;
        }

        let quantity_from_this_batch = remaining_to_load.min(batch.remaining_quantity);

        // Insert truck load item
        let load_item = sqlx::query!(
            r#"INSERT INTO truck_load_items (truck_load_id, batch_id, quantity_loaded)
            VALUES ($1, $2, $3)
            RETURNING id, truck_load_id, batch_id, quantity_loaded, quantity_sold, quantity_returned, created_at"#,
            truck_load_id as i32,
            batch.id,
            quantity_from_this_batch
        )
        .fetch_one(&mut **tx)
        .await?;

        // Deduct loaded quantity from batch remaining_quantity
        sqlx::query!(
            r#"UPDATE batches 
            SET remaining_quantity = remaining_quantity - $2
            WHERE id = $1"#,
            batch.id,
            quantity_from_this_batch
        )
        .execute(&mut **tx)
        .await?;

        loaded_items.push(TruckLoadItemResponse {
            id: load_item.id,
            batch_id: batch.id,
            batch_number: batch.batch_number,
            product_id: product_id,
            product_name: batch.product_name,
            expiry_date: batch.expiry_date,
            quantity_loaded: load_item.quantity_loaded,
            quantity_sold: load_item.quantity_sold,
            quantity_returned: load_item.quantity_returned,
            quantity_lost_damaged: 0, // Status is 'loaded', losses not determined yet
        });

        remaining_to_load -= quantity_from_this_batch;
    }

    Ok(loaded_items)
}
