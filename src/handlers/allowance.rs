use axum::{extract::State, Json, Extension};
use axum::http::StatusCode;
use crate::state::AppState;
use crate::error::AppError;
use crate::dtos::allowance::{
    CreateTransportAllowanceRequest, AllocateToTrucksRequest,
    UpdateTruckAllocationRequest, TransportAllowanceResponse,
    TruckAllocationResponse, AllowanceSummary,
};
use crate::middleware::auth::AuthContext;

pub async fn create_allowance(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateTransportAllowanceRequest>,
) -> Result<(StatusCode, Json<TransportAllowanceResponse>), AppError> {
    if auth.role != "manager" {
        return Err(AppError::forbidden("Only managers can create allowances"));
    }

    if req.total_allowance <= 0.0 {
        return Err(AppError::validation("Total allowance must be greater than 0"));
    }

    let allowance = sqlx::query!(
        r#"INSERT INTO transport_allowances (allowance_date, total_allowance, notes, created_by)
        VALUES ($1, $2::FLOAT8, $3, $4)
        RETURNING id, allowance_date, (total_allowance)::FLOAT8 as "total_allowance!", 
                  (allocated_amount)::FLOAT8 as "allocated_amount!", status, notes, created_at, updated_at"#,
        req.allowance_date,
        req.total_allowance,
        req.notes,
        auth.user_id
    )
    .fetch_one(&db_pool)
    .await
    .map_err(|e| {
        if let Some(db) = e.as_database_error() {
            if db.code().as_deref() == Some("23505") {
                if db.constraint() == Some("unique_allowance_date") {
                    return AppError::conflict("Allowance for this date already exists");
                }
            }
        }
        AppError::db(e)
    })?;

    Ok((
        StatusCode::CREATED,
        Json(TransportAllowanceResponse {
            id: allowance.id,
            allowance_date: allowance.allowance_date,
            total_allowance: allowance.total_allowance,
            allocated_amount: allowance.allocated_amount,
            remaining_amount: allowance.total_allowance - allowance.allocated_amount,
            status: allowance.status.unwrap_or_else(|| "pending".to_string()),
            notes: allowance.notes,
            created_by_username: auth.username.clone(),
            truck_allocations: vec![],
            created_at: allowance.created_at.unwrap(),
            updated_at: allowance.updated_at.unwrap(),
        }),
    ))
}

pub async fn allocate_to_trucks(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<AllocateToTrucksRequest>,
) -> Result<Json<TransportAllowanceResponse>, AppError> {
    if auth.role != "manager" {
        return Err(AppError::forbidden("Only managers can allocate allowances"));
    }

    if req.allocations.is_empty() {
        return Err(AppError::validation("At least one truck allocation is required"));
    }

    // Start transaction
    let mut tx = db_pool.begin().await?;

    // Get allowance and check status
    let allowance = sqlx::query!(
        r#"SELECT id, allowance_date, (total_allowance)::FLOAT8 as "total_allowance!", 
           (allocated_amount)::FLOAT8 as "allocated_amount!", status
        FROM transport_allowances
        WHERE id = $1"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::not_found("Allowance not found"))?;

    if allowance.status.as_deref() == Some("finalized") {
        return Err(AppError::validation("Cannot allocate to finalized allowance"));
    }

    // Calculate total new allocations
    let total_new_allocations: f64 = req.allocations.iter().map(|a| a.amount).sum();

    // Check if total allocation exceeds total allowance
    if allowance.allocated_amount + total_new_allocations > allowance.total_allowance {
        return Err(AppError::validation(&format!(
            "Total allocation ({}) would exceed total allowance ({}). Already allocated: {}, Remaining: {}",
            allowance.allocated_amount + total_new_allocations,
            allowance.total_allowance,
            allowance.allocated_amount,
            allowance.total_allowance - allowance.allocated_amount
        )));
    }

    // Validate each allocation
    for allocation in &req.allocations {
        if allocation.amount <= 0.0 {
            return Err(AppError::validation("Allocation amount must be greater than 0"));
        }

        if let Some(distance) = allocation.distance_covered {
            if distance < 0.0 {
                return Err(AppError::validation("Distance covered cannot be negative"));
            }
        }

        // Check if truck exists and get max limit
        let truck = sqlx::query!(
            r#"SELECT id, truck_number, is_active, (max_allowance_limit)::FLOAT8 as "max_allowance_limit!"
            FROM trucks
            WHERE id = $1"#,
            allocation.truck_id
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::not_found(&format!("Truck {} not found", allocation.truck_id)))?;

        if !truck.is_active {
            return Err(AppError::validation(&format!("Truck {} is not active", truck.truck_number)));
        }

        // Check if amount exceeds truck's max limit
        if allocation.amount > truck.max_allowance_limit {
            return Err(AppError::validation(&format!(
                "Allocation amount ({}) exceeds truck {}'s max limit ({})",
                allocation.amount, truck.truck_number, truck.max_allowance_limit
            )));
        }

        // Check if truck already has allocation for this allowance
        let existing = sqlx::query_scalar!(
            r#"SELECT EXISTS(
                SELECT 1 FROM truck_allowances 
                WHERE transport_allowance_id = $1 AND truck_id = $2
            ) as "exists!""#,
            id,
            allocation.truck_id
        )
        .fetch_one(&mut *tx)
        .await?;

        if existing {
            return Err(AppError::conflict(&format!("Truck {} already has an allocation for this date", truck.truck_number)));
        }
    }

    // Insert all allocations
    for allocation in &req.allocations {
        sqlx::query!(
            r#"INSERT INTO truck_allowances (transport_allowance_id, truck_id, amount, distance_covered, notes)
            VALUES ($1, $2, $3::FLOAT8, $4::FLOAT8, $5)"#,
            id,
            allocation.truck_id,
            allocation.amount,
            allocation.distance_covered,
            allocation.notes
        )
        .execute(&mut *tx)
        .await?;
    }

    // Update status to 'allocated'
    sqlx::query!(
        r#"UPDATE transport_allowances SET status = 'allocated' WHERE id = $1"#,
        id
    )
    .execute(&mut *tx)
    .await?;

    // Commit transaction
    tx.commit().await?;

    // Fetch and return updated allowance
    fetch_allowance_by_id(&db_pool, id).await.map(Json)
}

pub async fn update_truck_allocation(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path((allowance_id, truck_id)): axum::extract::Path<(i64, i64)>,
    Json(req): Json<UpdateTruckAllocationRequest>,
) -> Result<Json<TransportAllowanceResponse>, AppError> {
    if auth.role != "manager" {
        return Err(AppError::forbidden("Only managers can update allocations"));
    }

    if req.amount <= 0.0 {
        return Err(AppError::validation("Allocation amount must be greater than 0"));
    }

    if let Some(distance) = req.distance_covered {
        if distance < 0.0 {
            return Err(AppError::validation("Distance covered cannot be negative"));
        }
    }

    // Start transaction
    let mut tx = db_pool.begin().await?;

    // Check allowance status
    let allowance = sqlx::query!(
        r#"SELECT status, (total_allowance)::FLOAT8 as "total_allowance!", (allocated_amount)::FLOAT8 as "allocated_amount!"
        FROM transport_allowances WHERE id = $1"#,
        allowance_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::not_found("Allowance not found"))?;

    if allowance.status.as_deref() == Some("finalized") {
        return Err(AppError::validation("Cannot update finalized allowance"));
    }

    // Get current allocation
    let current_allocation = sqlx::query!(
        r#"SELECT (amount)::FLOAT8 as "amount!" FROM truck_allowances
        WHERE transport_allowance_id = $1 AND truck_id = $2"#,
        allowance_id,
        truck_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::not_found("Truck allocation not found"))?;

    // Get truck max limit
    let truck = sqlx::query!(
        r#"SELECT (max_allowance_limit)::FLOAT8 as "max_allowance_limit!" FROM trucks WHERE id = $1"#,
        truck_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::not_found("Truck not found"))?;

    // Check if new amount exceeds max limit
    if req.amount > truck.max_allowance_limit {
        return Err(AppError::validation(&format!(
            "Allocation amount ({}) exceeds truck's max limit ({})",
            req.amount, truck.max_allowance_limit
        )));
    }

    // Calculate new total allocated (subtract old, add new)
    let new_total_allocated = allowance.allocated_amount - current_allocation.amount + req.amount;

    if new_total_allocated > allowance.total_allowance {
        return Err(AppError::validation(&format!(
            "Updated allocation would exceed total allowance. Available: {}",
            allowance.total_allowance - (allowance.allocated_amount - current_allocation.amount)
        )));
    }

    // Update allocation
    sqlx::query!(
        r#"UPDATE truck_allowances
        SET amount = $3::FLOAT8, distance_covered = $4::FLOAT8, notes = $5
        WHERE transport_allowance_id = $1 AND truck_id = $2"#,
        allowance_id,
        truck_id,
        req.amount,
        req.distance_covered,
        req.notes
    )
    .execute(&mut *tx)
    .await?;

    // Commit transaction
    tx.commit().await?;

    // Fetch and return updated allowance
    fetch_allowance_by_id(&db_pool, allowance_id).await.map(Json)
}

pub async fn finalize_allowance(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<TransportAllowanceResponse>, AppError> {
    if auth.role != "manager" {
        return Err(AppError::forbidden("Only managers can finalize allowances"));
    }

    let result = sqlx::query!(
        r#"UPDATE transport_allowances
        SET status = 'finalized'
        WHERE id = $1 AND status != 'finalized'
        RETURNING id"#,
        id
    )
    .fetch_optional(&db_pool)
    .await?
    .ok_or_else(|| AppError::not_found("Allowance not found or already finalized"))?;

    fetch_allowance_by_id(&db_pool, result.id).await.map(Json)
}

pub async fn get_allowance(
    State(AppState { db_pool }): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<TransportAllowanceResponse>, AppError> {
    fetch_allowance_by_id(&db_pool, id).await.map(Json)
}

pub async fn list_allowances(
    State(AppState { db_pool }): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<AllowanceSummary>>, AppError> {
    let status = params.get("status");
    let start_date = params.get("start_date").and_then(|s| s.parse::<chrono::NaiveDate>().ok());
    let end_date = params.get("end_date").and_then(|s| s.parse::<chrono::NaiveDate>().ok());

    let mut query_str = String::from(
        r#"SELECT 
            id, allowance_date, 
            (total_allowance)::FLOAT8 as total_allowance,
            (allocated_amount)::FLOAT8 as allocated_amount,
            (total_allowance - allocated_amount)::FLOAT8 as remaining_amount,
            status,
            (truck_count)::INT as truck_count,
            created_by_username
        FROM allowance_summary
        WHERE 1=1"#
    );

    if status.is_some() {
        query_str.push_str(" AND status = $1");
    }
    if start_date.is_some() {
        let param_num = if status.is_some() { 2 } else { 1 };
        query_str.push_str(&format!(" AND allowance_date >= ${}", param_num));
    }
    if end_date.is_some() {
        let param_num = if status.is_some() && start_date.is_some() { 3 }
                       else if status.is_some() || start_date.is_some() { 2 }
                       else { 1 };
        query_str.push_str(&format!(" AND allowance_date <= ${}", param_num));
    }

    query_str.push_str(" ORDER BY allowance_date DESC");

    let mut query = sqlx::query_as::<_, (i64, chrono::NaiveDate, f64, f64, f64, String, i32, String)>(&query_str);

    if let Some(s) = status {
        query = query.bind(s);
    }
    if let Some(d) = start_date {
        query = query.bind(d);
    }
    if let Some(d) = end_date {
        query = query.bind(d);
    }

    let allowances = query.fetch_all(&db_pool).await?;

    Ok(Json(
        allowances
            .into_iter()
            .map(|(id, allowance_date, total_allowance, allocated_amount, remaining_amount, status, truck_count, created_by_username)| {
                AllowanceSummary {
                    id,
                    allowance_date,
                    total_allowance,
                    allocated_amount,
                    remaining_amount,
                    status,
                    truck_count,
                    created_by_username,
                }
            })
            .collect(),
    ))
}

pub async fn delete_allowance(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<StatusCode, AppError> {
    if auth.role != "manager" {
        return Err(AppError::forbidden("Only managers can delete allowances"));
    }

    let result = sqlx::query!(
        r#"DELETE FROM transport_allowances
        WHERE id = $1 AND status = 'pending'
        RETURNING id"#,
        id
    )
    .fetch_optional(&db_pool)
    .await?;

    if result.is_none() {
        return Err(AppError::validation("Can only delete pending allowances"));
    }

    Ok(StatusCode::NO_CONTENT)
}

// Helper function to fetch full allowance details
async fn fetch_allowance_by_id(
    db_pool: &sqlx::PgPool,
    id: i64,
) -> Result<TransportAllowanceResponse, AppError> {
    // Fetch allowance header
    let allowance = sqlx::query!(
        r#"SELECT 
            ta.id, ta.allowance_date,
            (ta.total_allowance)::FLOAT8 as "total_allowance!",
            (ta.allocated_amount)::FLOAT8 as "allocated_amount!",
            ta.status, ta.notes, ta.created_at, ta.updated_at,
            u.username as "created_by_username!"
        FROM transport_allowances ta
        JOIN users u ON ta.created_by = u.id
        WHERE ta.id = $1"#,
        id
    )
    .fetch_optional(db_pool)
    .await?
    .ok_or_else(|| AppError::not_found("Allowance not found"))?;

    // Fetch truck allocations
    let allocations_data = sqlx::query!(
        r#"SELECT 
            tka.id, tka.truck_id,
            (tka.amount)::FLOAT8 as "amount!",
            (tka.distance_covered)::FLOAT8 as distance_covered,
            tka.notes, tka.created_at,
            t.truck_number,
            (t.max_allowance_limit)::FLOAT8 as "max_allowance_limit!",
            u.username as "driver_username?"
        FROM truck_allowances tka
        JOIN trucks t ON tka.truck_id = t.id
        LEFT JOIN users u ON t.driver_id = u.id
        WHERE tka.transport_allowance_id = $1
        ORDER BY t.truck_number"#,
        id
    )
    .fetch_all(db_pool)
    .await?;

    let truck_allocations: Vec<TruckAllocationResponse> = allocations_data
        .into_iter()
        .map(|alloc| TruckAllocationResponse {
            id: alloc.id,
            truck_id: alloc.truck_id,
            truck_number: alloc.truck_number,
            driver_username: alloc.driver_username,
            max_limit: alloc.max_allowance_limit,
            amount: alloc.amount,
            distance_covered: alloc.distance_covered,
            notes: alloc.notes,
            created_at: alloc.created_at.unwrap(),
        })
        .collect();

    Ok(TransportAllowanceResponse {
        id: allowance.id,
        allowance_date: allowance.allowance_date,
        total_allowance: allowance.total_allowance,
        allocated_amount: allowance.allocated_amount,
        remaining_amount: allowance.total_allowance - allowance.allocated_amount,
        status: allowance.status.unwrap_or_else(|| "pending".to_string()),
        notes: allowance.notes,
        created_by_username: allowance.created_by_username,
        truck_allocations,
        created_at: allowance.created_at.unwrap(),
        updated_at: allowance.updated_at.unwrap(),
    })
}
