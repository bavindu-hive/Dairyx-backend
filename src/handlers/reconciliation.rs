use axum::{extract::{State, Path}, Json, Extension};
use chrono::NaiveDate;
use sqlx::{PgPool, Row};
use crate::{
    state::AppState,
    error::AppError,
    middleware::auth::AuthContext,
    dtos::reconciliation::*,
};

// ==================== Start Reconciliation ====================

pub async fn start_reconciliation(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<StartReconciliationRequest>,
) -> Result<Json<ReconciliationResponse>, AppError> {
    // Only managers can start reconciliation
    if auth.role != "manager" {
        return Err(AppError::forbidden("Only managers can start reconciliation"));
    }

    let mut tx = db_pool.begin().await?;

    // Check if reconciliation already exists for this date
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM daily_reconciliations WHERE reconciliation_date = $1)",
        req.reconciliation_date
    ).fetch_one(&mut *tx).await?.unwrap_or(false);

    if exists {
        return Err(AppError::conflict("Reconciliation already exists for this date"));
    }

    // Count trucks that went out on this date
    let trucks_out = sqlx::query_scalar!(
        r#"SELECT COUNT(DISTINCT truck_id)::INT as "count!" FROM truck_loads WHERE load_date = $1"#,
        req.reconciliation_date
    ).fetch_one(&mut *tx).await?;

    // Create reconciliation record
    let rec = sqlx::query!(
        r#"INSERT INTO daily_reconciliations 
           (reconciliation_date, status, trucks_out, started_by, notes)
           VALUES ($1, 'in_progress', $2, $3, $4)
           RETURNING id, started_at"#,
        req.reconciliation_date,
        trucks_out,
        auth.user_id as i32,
        req.notes
    ).fetch_one(&mut *tx).await?;

    // Get all truck loads for this date and create reconciliation_items
    // We need to get the driver from sales since truck_loads doesn't store driver_id
    let truck_loads = sqlx::query!(
        r#"SELECT 
            tl.id as truck_load_id,
            tl.truck_id,
            t.truck_number,
            COALESCE(s.user_id, tl.loaded_by) as "driver_id!",
            COALESCE(u.username, u2.username) as "driver_username!",
            COALESCE(SUM(tli.quantity_loaded), 0)::INT as "items_loaded!"
           FROM truck_loads tl
           JOIN trucks t ON tl.truck_id = t.id
           LEFT JOIN sales s ON s.truck_load_id = tl.id AND s.sale_date = $1
           LEFT JOIN users u ON s.user_id = u.id
           LEFT JOIN users u2 ON tl.loaded_by = u2.id
           LEFT JOIN truck_load_items tli ON tl.id = tli.truck_load_id
           WHERE tl.load_date = $1
           GROUP BY tl.id, tl.truck_id, t.truck_number, s.user_id, tl.loaded_by, u.username, u2.username"#,
        req.reconciliation_date
    ).fetch_all(&mut *tx).await?;

    let mut truck_items = Vec::new();

    for tl in truck_loads {
        // Get sales and payments for this truck on this date
        let sales_data = sqlx::query!(
            r#"SELECT 
                COALESCE(SUM(si.quantity), 0)::FLOAT8 as "items_sold!",
                COALESCE(SUM(si.quantity * p.commission_per_unit::FLOAT8), 0)::FLOAT8 as "commission!",
                COALESCE(SUM(s.total_amount::FLOAT8), 0)::FLOAT8 as "sales_amount!",
                COALESCE(SUM(s.amount_paid::FLOAT8), 0)::FLOAT8 as "payments!"
               FROM sales s
               LEFT JOIN sale_items si ON s.id = si.sale_id
               LEFT JOIN batches b ON si.batch_id = b.id
               LEFT JOIN products p ON b.product_id = p.id
               WHERE s.truck_id = $1 AND s.sale_date = $2"#,
            tl.truck_id,
            req.reconciliation_date
        ).fetch_one(&mut *tx).await?;

        // Get allowance for this truck
        let allowance = sqlx::query_scalar!(
            r#"SELECT COALESCE((ta.amount)::FLOAT8, 0) as "allowance!"
               FROM transport_allowances tallow
               JOIN truck_allowances ta ON tallow.id = ta.transport_allowance_id
               WHERE tallow.allowance_date = $1 AND ta.truck_id = $2"#,
            req.reconciliation_date,
            tl.truck_id
        ).fetch_optional(&mut *tx).await?.unwrap_or(0.0);


        let items_loaded = tl.items_loaded as f64;
        let items_sold = sales_data.items_sold;
        let pending_payments = sales_data.sales_amount - sales_data.payments;

        // Create reconciliation item
        let item = sqlx::query!(
            r#"INSERT INTO reconciliation_items 
               (reconciliation_id, truck_id, driver_id, truck_load_id, 
                items_loaded, items_sold, items_returned, items_discarded,
                sales_amount, commission_earned, allowance_received, 
                payments_collected, pending_payments)
               VALUES ($1, $2, $3, $4, ($5)::FLOAT8::NUMERIC, ($6)::FLOAT8::NUMERIC, 0, 0, 
                       ($7)::FLOAT8::NUMERIC, ($8)::FLOAT8::NUMERIC, ($9)::FLOAT8::NUMERIC, 
                       ($10)::FLOAT8::NUMERIC, ($11)::FLOAT8::NUMERIC)
               RETURNING id"#,
            rec.id,
            tl.truck_id as i32,
            tl.driver_id as i32,
            tl.truck_load_id as i32,
            items_loaded,
            items_sold,
            sales_data.sales_amount,
            sales_data.commission,
            allowance,
            sales_data.payments,
            pending_payments
        ).fetch_one(&mut *tx).await?;

        truck_items.push(TruckVerificationItem {
            id: item.id as i64,
            truck_id: tl.truck_id as i64,
            truck_number: tl.truck_number,
            driver_id: tl.driver_id as i64,
            driver_username: tl.driver_username,
            truck_load_id: tl.truck_load_id as i64,
            items_loaded,
            items_sold,
            items_returned: 0.0,
            items_discarded: 0.0,
            is_verified: false,
            has_discrepancy: false,
            discrepancy_notes: None,
            sales_amount: sales_data.sales_amount,
            commission_earned: sales_data.commission,
            allowance_received: allowance,
            payments_collected: sales_data.payments,
            pending_payments,
            verified_by: None,
            verified_at: None,
        });
    }

    tx.commit().await?;

    Ok(Json(ReconciliationResponse {
        id: rec.id as i64,
        reconciliation_date: req.reconciliation_date,
        status: "in_progress".to_string(),
        trucks_out,
        trucks_verified: 0,
        total_items_loaded: 0.0,
        total_items_sold: 0.0,
        total_items_returned: 0.0,
        total_items_discarded: 0.0,
        total_sales_amount: 0.0,
        total_commission_earned: 0.0,
        total_allowance_allocated: 0.0,
        total_payments_collected: 0.0,
        pending_payments: 0.0,
        net_profit: 0.0,
        started_by: Some(auth.user_id),
        started_by_username: Some(auth.username.clone()),
        started_at: rec.started_at,
        finalized_by: None,
        finalized_by_username: None,
        finalized_at: None,
        notes: req.notes,
        truck_items,
    }))
}

// ==================== Verify Truck Return ====================

pub async fn verify_truck_return(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path((date, truck_id)): Path<(NaiveDate, i64)>,
    Json(req): Json<VerifyTruckReturnRequest>,
) -> Result<Json<TruckVerificationItem>, AppError> {
    // Only managers can verify returns
    if auth.role != "manager" {
        return Err(AppError::forbidden("Only managers can verify truck returns"));
    }

    let mut tx = db_pool.begin().await?;

    // Get reconciliation for this date
    let rec = sqlx::query!(
        r#"SELECT id, (status)::TEXT as "status!" FROM daily_reconciliations WHERE reconciliation_date = $1"#,
        date
    ).fetch_optional(&mut *tx).await?
        .ok_or_else(|| AppError::not_found("Reconciliation not found for this date"))?;

    if rec.status != "in_progress" {
        return Err(AppError::conflict("Reconciliation is not in progress"));
    }

    // Get reconciliation item for this truck
    let item = sqlx::query!(
        r#"SELECT id, (items_loaded)::FLOAT8 as "items_loaded!", (items_sold)::FLOAT8 as "items_sold!" 
           FROM reconciliation_items 
           WHERE reconciliation_id = $1 AND truck_id = $2"#,
        rec.id,
        truck_id as i32
    ).fetch_optional(&mut *tx).await?
        .ok_or_else(|| AppError::not_found("Truck not found in this reconciliation"))?;

    // Calculate totals
    let total_returned: f64 = req.items_returned.iter().map(|i| i.quantity as f64).sum();
    let total_discarded: f64 = req.items_discarded.iter().map(|i| i.quantity as f64).sum();
    
    let items_loaded = item.items_loaded;
    let items_sold = item.items_sold;
    let expected_return = items_loaded - items_sold;
    let actual_return = total_returned + total_discarded;

    // Check for discrepancy
    let has_discrepancy = (expected_return - actual_return).abs() > 0.01;

    // Update reconciliation item
    sqlx::query!(
        r#"UPDATE reconciliation_items 
           SET items_returned = ($1)::FLOAT8::NUMERIC,
               items_discarded = ($2)::FLOAT8::NUMERIC,
               is_verified = true,
               has_discrepancy = $3,
               discrepancy_notes = $4,
               verified_by = $5,
               verified_at = NOW()
           WHERE id = $6"#,
        total_returned,
        total_discarded,
        has_discrepancy,
        req.discrepancy_notes,
        auth.user_id as i32,
        item.id
    ).execute(&mut *tx).await?;

    // Update reconciliation trucks_verified count
    sqlx::query!(
        r#"UPDATE daily_reconciliations 
           SET trucks_verified = (
               SELECT COUNT(*)::INT FROM reconciliation_items 
               WHERE reconciliation_id = $1 AND is_verified = true
           )
           WHERE id = $1"#,
        rec.id
    ).execute(&mut *tx).await?;

    tx.commit().await?;

    // Fetch updated item details
    fetch_truck_verification_item(&db_pool, rec.id as i64, truck_id).await
}

// ==================== Finalize Reconciliation ====================

pub async fn finalize_reconciliation(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(date): Path<NaiveDate>,
) -> Result<Json<ReconciliationResponse>, AppError> {
    // Only managers can finalize
    if auth.role != "manager" {
        return Err(AppError::forbidden("Only managers can finalize reconciliation"));
    }

    let mut tx = db_pool.begin().await?;

    // Get reconciliation
    let rec = sqlx::query!(
        r#"SELECT id, (status)::TEXT as "status!", trucks_out, trucks_verified FROM daily_reconciliations 
           WHERE reconciliation_date = $1"#,
        date
    ).fetch_optional(&mut *tx).await?
        .ok_or_else(|| AppError::not_found("Reconciliation not found"))?;

    if rec.status == "finalized" {
        return Err(AppError::conflict("Reconciliation already finalized"));
    }

    // Check if all trucks are verified
    if rec.trucks_verified < rec.trucks_out {
        return Err(AppError::validation(format!(
            "Not all trucks verified. {}/{} trucks verified",
            rec.trucks_verified, rec.trucks_out
        )));
    }

    // Get all reconciliation items
    let items = sqlx::query!(
        r#"SELECT 
            ri.truck_load_id,
            (ri.items_returned)::FLOAT8 as "items_returned!",
            (ri.items_discarded)::FLOAT8 as "items_discarded!",
            (ri.sales_amount)::FLOAT8 as "sales_amount!",
            (ri.commission_earned)::FLOAT8 as "commission_earned!",
            (ri.allowance_received)::FLOAT8 as "allowance_received!",
            (ri.payments_collected)::FLOAT8 as "payments_collected!",
            (ri.pending_payments)::FLOAT8 as "pending_payments!"
           FROM reconciliation_items ri
           WHERE ri.reconciliation_id = $1"#,
        rec.id
    ).fetch_all(&mut *tx).await?;

    // Return stock to batches and create stock movements
    for item in &items {
        if item.items_returned > 0.0 {
            // Get truck load items to know which batches to return stock to
            let truck_items = sqlx::query!(
                r#"SELECT 
                    tli.batch_id,
                    b.product_id,
                    tli.quantity_loaded as loaded,
                    tli.quantity_sold as sold,
                    (tli.quantity_loaded - tli.quantity_sold) as remaining
                   FROM truck_load_items tli
                   JOIN batches b ON tli.batch_id = b.id
                   WHERE tli.truck_load_id = $1 AND (tli.quantity_loaded - tli.quantity_sold) > 0"#,
                item.truck_load_id as i32
            ).fetch_all(&mut *tx).await?;

            for ti in truck_items {
                let return_qty = ti.remaining.unwrap_or(0);
                
                // Update batch remaining_quantity
                sqlx::query!(
                    r#"UPDATE batches 
                       SET remaining_quantity = remaining_quantity + $1
                       WHERE id = $2"#,
                    return_qty,
                    ti.batch_id
                ).execute(&mut *tx).await?;

                // Log stock movement
                sqlx::query!(
                    r#"INSERT INTO stock_movements 
                       (batch_id, product_id, movement_type, quantity, 
                        reference_type, reference_id, notes, created_by, movement_date)
                       VALUES ($1, $2, 'truck_return_in', $3, 'reconciliation', $4, 
                               'Truck return - end of day reconciliation', $5, $6)"#,
                    ti.batch_id as i32,
                    ti.product_id as i32,
                    return_qty as f64,
                    rec.id as i32,
                    auth.user_id as i32,
                    date
                ).execute(&mut *tx).await?;
            }
        }
    }

    // Calculate totals
    let totals = sqlx::query!(
        r#"SELECT 
            COALESCE(SUM(items_loaded), 0)::FLOAT8 as "total_loaded!",
            COALESCE(SUM(items_sold), 0)::FLOAT8 as "total_sold!",
            COALESCE(SUM(items_returned), 0)::FLOAT8 as "total_returned!",
            COALESCE(SUM(items_discarded), 0)::FLOAT8 as "total_discarded!",
            COALESCE(SUM(sales_amount), 0)::FLOAT8 as "total_sales!",
            COALESCE(SUM(commission_earned), 0)::FLOAT8 as "total_commission!",
            COALESCE(SUM(allowance_received), 0)::FLOAT8 as "total_allowance!",
            COALESCE(SUM(payments_collected), 0)::FLOAT8 as "total_payments!",
            COALESCE(SUM(pending_payments), 0)::FLOAT8 as "total_pending!"
           FROM reconciliation_items
           WHERE reconciliation_id = $1"#,
        rec.id
    ).fetch_one(&mut *tx).await?;

    let net_profit = totals.total_commission - totals.total_allowance;

    // Update reconciliation with totals and mark as finalized
    sqlx::query!(
        r#"UPDATE daily_reconciliations 
           SET status = 'finalized',
               total_items_loaded = ($1)::FLOAT8::NUMERIC,
               total_items_sold = ($2)::FLOAT8::NUMERIC,
               total_items_returned = ($3)::FLOAT8::NUMERIC,
               total_items_discarded = ($4)::FLOAT8::NUMERIC,
               total_sales_amount = ($5)::FLOAT8::NUMERIC,
               total_commission_earned = ($6)::FLOAT8::NUMERIC,
               total_allowance_allocated = ($7)::FLOAT8::NUMERIC,
               total_payments_collected = ($8)::FLOAT8::NUMERIC,
               pending_payments = ($9)::FLOAT8::NUMERIC,
               net_profit = ($10)::FLOAT8::NUMERIC,
               finalized_by = $11,
               finalized_at = NOW()
           WHERE id = $12"#,
        totals.total_loaded,
        totals.total_sold,
        totals.total_returned,
        totals.total_discarded,
        totals.total_sales,
        totals.total_commission,
        totals.total_allowance,
        totals.total_payments,
        totals.total_pending,
        net_profit,
        auth.user_id as i32,
        rec.id
    ).execute(&mut *tx).await?;

    tx.commit().await?;

    // Fetch and return full reconciliation response
    Ok(Json(fetch_reconciliation(&db_pool, date).await?))
}

// ==================== Get Reconciliation ====================

pub async fn get_reconciliation(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(date): Path<NaiveDate>,
) -> Result<Json<ReconciliationResponse>, AppError> {
    // Managers can see all, drivers can only see if they're involved
    fetch_reconciliation(&db_pool, date).await.map(Json)
}

// ==================== List Reconciliations ====================

pub async fn list_reconciliations(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<ReconciliationSummary>>, AppError> {
    // Only managers can list all reconciliations
    if auth.role != "manager" {
        return Err(AppError::forbidden("Only managers can list reconciliations"));
    }

    let status = params.get("status");
    let start_date = params.get("start_date").and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
    let end_date = params.get("end_date").and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());

    let mut query = String::from(
        r#"SELECT 
            id, reconciliation_date, status, trucks_out, trucks_verified,
            (net_profit)::FLOAT8 as net_profit,
            CASE WHEN net_profit >= 0 THEN 'profit' ELSE 'loss' END as profit_status,
            started_at, finalized_at
           FROM daily_reconciliations
           WHERE 1=1"#
    );

    if let Some(s) = status {
        query.push_str(&format!(" AND status = '{}'", s));
    }
    if let Some(sd) = start_date {
        query.push_str(&format!(" AND reconciliation_date >= '{}'", sd));
    }
    if let Some(ed) = end_date {
        query.push_str(&format!(" AND reconciliation_date <= '{}'", ed));
    }

    query.push_str(" ORDER BY reconciliation_date DESC");

    let rows = sqlx::query(&query).fetch_all(&db_pool).await?;

    let summaries: Vec<ReconciliationSummary> = rows.iter().map(|row| {
        ReconciliationSummary {
            id: row.get("id"),
            reconciliation_date: row.get("reconciliation_date"),
            status: row.get("status"),
            trucks_out: row.get("trucks_out"),
            trucks_verified: row.get("trucks_verified"),
            net_profit: row.get("net_profit"),
            profit_status: row.get("profit_status"),
            started_at: row.get("started_at"),
            finalized_at: row.get("finalized_at"),
        }
    }).collect();

    Ok(Json(summaries))
}

// ==================== Helper Functions ====================

async fn fetch_reconciliation(db_pool: &PgPool, date: NaiveDate) -> Result<ReconciliationResponse, AppError> {
    let rec = sqlx::query!(
        r#"SELECT 
            dr.id, dr.reconciliation_date, (dr.status)::TEXT as "status!",
            dr.trucks_out, dr.trucks_verified,
            (dr.total_items_loaded)::FLOAT8 as "total_items_loaded!",
            (dr.total_items_sold)::FLOAT8 as "total_items_sold!",
            (dr.total_items_returned)::FLOAT8 as "total_items_returned!",
            (dr.total_items_discarded)::FLOAT8 as "total_items_discarded!",
            (dr.total_sales_amount)::FLOAT8 as "total_sales_amount!",
            (dr.total_commission_earned)::FLOAT8 as "total_commission_earned!",
            (dr.total_allowance_allocated)::FLOAT8 as "total_allowance_allocated!",
            (dr.total_payments_collected)::FLOAT8 as "total_payments_collected!",
            (dr.pending_payments)::FLOAT8 as "pending_payments!",
            (dr.net_profit)::FLOAT8 as "net_profit!",
            dr.started_by, su.username as "started_by_username?", dr.started_at,
            dr.finalized_by, fu.username as "finalized_by_username?", dr.finalized_at,
            dr.notes
           FROM daily_reconciliations dr
           LEFT JOIN users su ON dr.started_by = su.id
           LEFT JOIN users fu ON dr.finalized_by = fu.id
           WHERE dr.reconciliation_date = $1"#,
        date
    ).fetch_optional(db_pool).await?
        .ok_or_else(|| AppError::not_found("Reconciliation not found"))?;

    // Get truck items
    let items = sqlx::query!(
        r#"SELECT 
            ri.id, ri.truck_id, t.truck_number, ri.driver_id, u.username as driver_username,
            ri.truck_load_id,
            (ri.items_loaded)::FLOAT8 as "items_loaded!",
            (ri.items_sold)::FLOAT8 as "items_sold!",
            (ri.items_returned)::FLOAT8 as "items_returned!",
            (ri.items_discarded)::FLOAT8 as "items_discarded!",
            ri.is_verified, ri.has_discrepancy, ri.discrepancy_notes,
            (ri.sales_amount)::FLOAT8 as "sales_amount!",
            (ri.commission_earned)::FLOAT8 as "commission_earned!",
            (ri.allowance_received)::FLOAT8 as "allowance_received!",
            (ri.payments_collected)::FLOAT8 as "payments_collected!",
            (ri.pending_payments)::FLOAT8 as "pending_payments!",
            ri.verified_by, ri.verified_at
           FROM reconciliation_items ri
           JOIN trucks t ON ri.truck_id = t.id
           JOIN users u ON ri.driver_id = u.id
           WHERE ri.reconciliation_id = $1
           ORDER BY t.truck_number"#,
        rec.id
    ).fetch_all(db_pool).await?;

    let truck_items = items.into_iter().map(|item| TruckVerificationItem {
        id: item.id as i64,
        truck_id: item.truck_id as i64,
        truck_number: item.truck_number,
        driver_id: item.driver_id as i64,
        driver_username: item.driver_username,
        truck_load_id: item.truck_load_id as i64,
        items_loaded: item.items_loaded,
        items_sold: item.items_sold,
        items_returned: item.items_returned,
        items_discarded: item.items_discarded,
        is_verified: item.is_verified,
        has_discrepancy: item.has_discrepancy,
        discrepancy_notes: item.discrepancy_notes,
        sales_amount: item.sales_amount,
        commission_earned: item.commission_earned,
        allowance_received: item.allowance_received,
        payments_collected: item.payments_collected,
        pending_payments: item.pending_payments,
        verified_by: item.verified_by.map(|id| id as i64),
        verified_at: item.verified_at,
    }).collect();

    Ok(ReconciliationResponse {
        id: rec.id as i64,
        reconciliation_date: rec.reconciliation_date,
        status: rec.status,
        trucks_out: rec.trucks_out,
        trucks_verified: rec.trucks_verified,
        total_items_loaded: rec.total_items_loaded,
        total_items_sold: rec.total_items_sold,
        total_items_returned: rec.total_items_returned,
        total_items_discarded: rec.total_items_discarded,
        total_sales_amount: rec.total_sales_amount,
        total_commission_earned: rec.total_commission_earned,
        total_allowance_allocated: rec.total_allowance_allocated,
        total_payments_collected: rec.total_payments_collected,
        pending_payments: rec.pending_payments,
        net_profit: rec.net_profit,
        started_by: rec.started_by.map(|id| id as i64),
        started_by_username: rec.started_by_username,
        started_at: rec.started_at,
        finalized_by: rec.finalized_by.map(|id| id as i64),
        finalized_by_username: rec.finalized_by_username,
        finalized_at: rec.finalized_at,
        notes: rec.notes,
        truck_items,
    })
}

async fn fetch_truck_verification_item(
    db_pool: &PgPool,
    reconciliation_id: i64,
    truck_id: i64,
) -> Result<Json<TruckVerificationItem>, AppError> {
    let item = sqlx::query!(
        r#"SELECT 
            ri.id, ri.truck_id, t.truck_number, ri.driver_id, u.username as driver_username,
            ri.truck_load_id,
            (ri.items_loaded)::FLOAT8 as "items_loaded!",
            (ri.items_sold)::FLOAT8 as "items_sold!",
            (ri.items_returned)::FLOAT8 as "items_returned!",
            (ri.items_discarded)::FLOAT8 as "items_discarded!",
            ri.is_verified, ri.has_discrepancy, ri.discrepancy_notes,
            (ri.sales_amount)::FLOAT8 as "sales_amount!",
            (ri.commission_earned)::FLOAT8 as "commission_earned!",
            (ri.allowance_received)::FLOAT8 as "allowance_received!",
            (ri.payments_collected)::FLOAT8 as "payments_collected!",
            (ri.pending_payments)::FLOAT8 as "pending_payments!",
            ri.verified_by, ri.verified_at
           FROM reconciliation_items ri
           JOIN trucks t ON ri.truck_id = t.id
           JOIN users u ON ri.driver_id = u.id
           WHERE ri.reconciliation_id = $1 AND ri.truck_id = $2"#,
        reconciliation_id as i32,
        truck_id as i32
    ).fetch_optional(db_pool).await?
        .ok_or_else(|| AppError::not_found("Truck not found in reconciliation"))?;

    Ok(Json(TruckVerificationItem {
        id: item.id as i64,
        truck_id: item.truck_id as i64,
        truck_number: item.truck_number,
        driver_id: item.driver_id as i64,
        driver_username: item.driver_username,
        truck_load_id: item.truck_load_id as i64,
        items_loaded: item.items_loaded,
        items_sold: item.items_sold,
        items_returned: item.items_returned,
        items_discarded: item.items_discarded,
        is_verified: item.is_verified,
        has_discrepancy: item.has_discrepancy,
        discrepancy_notes: item.discrepancy_notes,
        sales_amount: item.sales_amount,
        commission_earned: item.commission_earned,
        allowance_received: item.allowance_received,
        payments_collected: item.payments_collected,
        pending_payments: item.pending_payments,
        verified_by: item.verified_by.map(|id| id as i64),
        verified_at: item.verified_at,
    }))
}
