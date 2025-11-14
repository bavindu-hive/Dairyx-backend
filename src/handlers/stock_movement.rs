use crate::{
    dtos::reconciliation::*, error::AppError, middleware::auth::AuthContext, state::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::NaiveDate;
use sqlx::Row;

// ==================== Get Batch Movements ====================

pub async fn get_batch_movements(
    State(AppState { db_pool }): State<AppState>,
    Extension(_auth): Extension<AuthContext>,
    Path(batch_id): Path<i64>,
) -> Result<Json<BatchMovementHistory>, AppError> {
    // Get batch details
    let batch = sqlx::query!(
        r#"SELECT b.id, b.batch_number, b.product_id, p.name as product_name,
                  b.quantity as initial_quantity, b.remaining_quantity
           FROM batches b
           JOIN products p ON b.product_id = p.id
           WHERE b.id = $1"#,
        batch_id
    )
    .fetch_optional(&db_pool)
    .await?
    .ok_or_else(|| AppError::not_found("Batch not found"))?;

    // Get all movements for this batch with running balance
    let movements = sqlx::query!(
        r#"SELECT 
            sm.id,
            sm.movement_type as "movement_type!: StockMovementType",
            (sm.quantity)::FLOAT8 as "quantity!",
            sm.reference_type::TEXT as "reference_type!",
            sm.reference_id,
            sm.notes,
            u.username as "created_by?",
            sm.movement_date,
            SUM(
                CASE 
                    WHEN sm.movement_type IN ('delivery_in', 'truck_return_in', 'adjustment') 
                    THEN (sm.quantity)::FLOAT8
                    ELSE -(sm.quantity)::FLOAT8
                END
            ) OVER (ORDER BY sm.created_at, sm.id) as "running_balance!"
           FROM stock_movements sm
           LEFT JOIN users u ON sm.created_by = u.id
           WHERE sm.batch_id = $1
           ORDER BY sm.created_at ASC, sm.id ASC"#,
        batch_id as i32
    )
    .fetch_all(&db_pool)
    .await?;

    let movement_details = movements
        .into_iter()
        .map(|m| StockMovementDetail {
            id: m.id,
            movement_type: m.movement_type,
            quantity: m.quantity,
            reference_type: m.reference_type,
            reference_id: m.reference_id,
            notes: m.notes,
            created_by: m.created_by,
            movement_date: m.movement_date,
            running_balance: m.running_balance,
        })
        .collect();

    Ok(Json(BatchMovementHistory {
        batch_id: batch.id,
        batch_number: batch.batch_number,
        product_id: batch.product_id,
        product_name: batch.product_name,
        initial_quantity: batch.initial_quantity,
        current_remaining: batch.remaining_quantity,
        movements: movement_details,
    }))
}

// ==================== Get Daily Movements ====================

pub async fn get_daily_movements(
    State(AppState { db_pool }): State<AppState>,
    Extension(_auth): Extension<AuthContext>,
    Path(date): Path<NaiveDate>,
) -> Result<Json<DailyStockSummary>, AppError> {
    let movements = sqlx::query!(
        r#"SELECT 
            sm.product_id,
            p.name as product_name,
            sm.movement_type as "movement_type!: StockMovementType",
            COUNT(*) as "transaction_count!",
            SUM((sm.quantity)::FLOAT8) as "total_quantity!"
           FROM stock_movements sm
           JOIN products p ON sm.product_id = p.id
           WHERE sm.movement_date = $1
           GROUP BY sm.product_id, p.name, sm.movement_type
           ORDER BY sm.product_id, sm.movement_type"#,
        date
    )
    .fetch_all(&db_pool)
    .await?;

    // Group by product
    let mut product_map: std::collections::HashMap<i64, (String, Vec<MovementTypeSummary>)> =
        std::collections::HashMap::new();

    for m in movements {
        let entry = product_map
            .entry(m.product_id as i64)
            .or_insert((m.product_name.clone(), Vec::new()));
        entry.1.push(MovementTypeSummary {
            movement_type: m.movement_type,
            transaction_count: m.transaction_count,
            total_quantity: m.total_quantity,
        });
    }

    let product_summaries = product_map
        .into_iter()
        .map(
            |(product_id, (product_name, movements))| ProductStockSummary {
                product_id,
                product_name,
                movements,
            },
        )
        .collect();

    Ok(Json(DailyStockSummary {
        movement_date: date,
        product_summaries,
    }))
}

// ==================== Get Product Movements ====================

pub async fn get_product_movements(
    State(AppState { db_pool }): State<AppState>,
    Extension(_auth): Extension<AuthContext>,
    Path(product_id): Path<i64>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<StockMovementResponse>>, AppError> {
    let start_date = params
        .get("start_date")
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
    let end_date = params
        .get("end_date")
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
    let movement_type = params.get("movement_type");

    let mut query = String::from(
        r#"SELECT 
            sm.id, sm.batch_id, sm.product_id, p.name as product_name,
            sm.movement_type::TEXT as movement_type,
            (sm.quantity)::FLOAT8 as quantity,
            sm.reference_type::TEXT as reference_type,
            sm.reference_id, sm.notes, sm.created_by,
            u.username as created_by_username,
            sm.movement_date, sm.created_at
           FROM stock_movements sm
           JOIN products p ON sm.product_id = p.id
           LEFT JOIN users u ON sm.created_by = u.id
           WHERE sm.product_id = "#,
    );
    query.push_str(&product_id.to_string());

    if let Some(sd) = start_date {
        query.push_str(&format!(" AND sm.movement_date >= '{}'", sd));
    }
    if let Some(ed) = end_date {
        query.push_str(&format!(" AND sm.movement_date <= '{}'", ed));
    }
    if let Some(mt) = movement_type {
        query.push_str(&format!(" AND sm.movement_type::TEXT = '{}'", mt));
    }

    query.push_str(" ORDER BY sm.created_at DESC");

    let rows = sqlx::query(&query).fetch_all(&db_pool).await?;

    let movements: Vec<StockMovementResponse> = rows
        .iter()
        .map(|row| {
            let movement_type_str: String = row.get("movement_type");
            let movement_type = match movement_type_str.as_str() {
                "delivery_in" => StockMovementType::DeliveryIn,
                "truck_load_out" => StockMovementType::TruckLoadOut,
                "sale_out" => StockMovementType::SaleOut,
                "truck_return_in" => StockMovementType::TruckReturnIn,
                "adjustment" => StockMovementType::Adjustment,
                "expired_out" => StockMovementType::ExpiredOut,
                _ => StockMovementType::Adjustment, // fallback
            };
            
            StockMovementResponse {
                id: row.get("id"),
                batch_id: row.get("batch_id"),
                product_id: row.get::<i32, _>("product_id") as i64,
                product_name: row.get("product_name"),
                movement_type,
                quantity: row.get("quantity"),
                reference_type: row.get("reference_type"),
                reference_id: row.get("reference_id"),
                notes: row.get("notes"),
                created_by: row.get::<Option<i32>, _>("created_by").map(|id| id as i64),
                created_by_username: row.get("created_by_username"),
                movement_date: row.get("movement_date"),
                created_at: row.get("created_at"),
            }
        })
        .collect();

    Ok(Json(movements))
}

// ==================== Create Stock Adjustment ====================

pub async fn create_stock_adjustment(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateStockAdjustmentRequest>,
) -> Result<(StatusCode, Json<StockMovementResponse>), AppError> {
    // Only managers can create adjustments
    if auth.role != "manager" {
        return Err(AppError::forbidden(
            "Only managers can create stock adjustments",
        ));
    }

    // Validate movement type - only adjustment and expired_out are allowed
    use crate::dtos::reconciliation::StockMovementType;
    match req.movement_type {
        StockMovementType::Adjustment | StockMovementType::ExpiredOut => {}
        _ => {
            return Err(AppError::validation(
                "movement_type must be 'adjustment' or 'expired_out'",
            ))
        }
    }

    // Validate quantity based on movement type
    match req.movement_type {
        StockMovementType::ExpiredOut => {
            if req.quantity <= 0.0 {
                return Err(AppError::validation(
                    "Quantity must be greater than 0 for expired_out",
                ));
            }
        }
        StockMovementType::Adjustment => {
            if req.quantity == 0.0 {
                return Err(AppError::validation("Adjustment quantity cannot be 0. Use positive for increase, negative for decrease"));
            }
        }
        _ => unreachable!(),
    }

    let mut tx = db_pool.begin().await?;

    // Verify batch exists and has enough quantity for removal
    let batch = sqlx::query!(
        r#"SELECT remaining_quantity, product_id FROM batches WHERE id = $1"#,
        req.batch_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::not_found("Batch not found"))?;

    if batch.product_id != req.product_id {
        return Err(AppError::validation("Product ID does not match batch"));
    }

    // Handle stock updates based on movement type
    match req.movement_type {
        StockMovementType::ExpiredOut => {
            // For removals, check if enough stock
            if batch.remaining_quantity < req.quantity as i32 {
                return Err(AppError::validation(format!(
                    "Insufficient stock. Available: {}, Requested: {}",
                    batch.remaining_quantity, req.quantity
                )));
            }
            // Decrease batch quantity
            sqlx::query!(
                r#"UPDATE batches SET remaining_quantity = remaining_quantity - $1 WHERE id = $2"#,
                req.quantity as i32,
                req.batch_id
            )
            .execute(&mut *tx)
            .await?;
        }
        StockMovementType::Adjustment => {
            // For adjustments, quantity can be positive (increase) or negative (decrease)
            // If decrease, check sufficient stock
            if req.quantity < 0.0 && batch.remaining_quantity < req.quantity.abs() as i32 {
                return Err(AppError::validation(format!(
                    "Insufficient stock. Available: {}, Requested decrease: {}",
                    batch.remaining_quantity,
                    req.quantity.abs()
                )));
            }
            // Update BOTH quantity and remaining_quantity to maintain constraint
            // remaining_quantity must always be <= quantity
            sqlx::query!(
                r#"UPDATE batches 
                   SET quantity = quantity + $1,
                       remaining_quantity = remaining_quantity + $1 
                   WHERE id = $2"#,
                req.quantity as i32,
                req.batch_id
            )
            .execute(&mut *tx)
            .await?;
        }
        _ => unreachable!(), // Already validated above
    }

    // Create stock movement
    let notes = format!("{} - {}", req.reason, req.notes.unwrap_or_default());

    // Insert with enum type - sqlx handles the conversion automatically
    let movement = sqlx::query_as::<_, (i32, NaiveDate, chrono::NaiveDateTime)>(
        r#"INSERT INTO stock_movements 
           (batch_id, product_id, movement_type, quantity, reference_type, reference_id, 
            notes, created_by, movement_date)
           VALUES ($1, $2, $3, $4, 'manual', $5, $6, $7, CURRENT_DATE)
           RETURNING id, movement_date, created_at"#,
    )
    .bind(req.batch_id as i32)
    .bind(req.product_id as i32)
    .bind(&req.movement_type) // Enum is automatically converted
    .bind(req.quantity)
    .bind(req.batch_id as i32) // Use batch_id as reference_id for manual adjustments
    .bind(notes.clone())
    .bind(auth.user_id as i32)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    // Fetch product name
    let product = sqlx::query!(r#"SELECT name FROM products WHERE id = $1"#, req.product_id)
        .fetch_one(&db_pool)
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(StockMovementResponse {
            id: movement.0,
            batch_id: req.batch_id as i32,
            product_id: req.product_id,
            product_name: product.name,
            movement_type: req.movement_type,
            quantity: req.quantity,
            reference_type: "manual".to_string(),
            reference_id: req.batch_id as i32, // Use batch_id as reference
            notes: Some(notes),
            created_by: Some(auth.user_id),
            created_by_username: Some(auth.username),
            movement_date: movement.1,
            created_at: movement.2,
        }),
    ))
}
