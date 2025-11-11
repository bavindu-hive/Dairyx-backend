use axum::{extract::State, Json};
use axum::http::StatusCode;
use crate::state::AppState;
use crate::error::AppError;
use crate::dtos::truck::{CreateTruckRequest, UpdateTruckRequest, TruckResponse, TruckSummary};
use crate::middleware::auth::AuthContext;
use axum::extract::Extension;

pub async fn create_truck(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateTruckRequest>,
) -> Result<(StatusCode, Json<TruckResponse>), AppError> {
    if auth.role != "manager" {
        return Err(AppError::forbidden("Only managers can create trucks"));
    }

    if req.truck_number.trim().is_empty() {
        return Err(AppError::validation("Truck number is required"));
    }

    // If driver_id provided, validate it's a driver (not manager)
    if let Some(driver_id) = req.driver_id {
        let driver = sqlx::query!(
            r#"SELECT role FROM users WHERE id = $1"#,
            driver_id
        )
        .fetch_optional(&db_pool)
        .await?
        .ok_or_else(|| AppError::not_found("Driver not found"))?;

        if driver.role != "driver" {
            return Err(AppError::validation("Only users with role 'driver' can be assigned to trucks"));
        }
    }

    let truck = sqlx::query!(
        r#"INSERT INTO trucks (truck_number, driver_id)
        VALUES ($1, $2)
        RETURNING id, truck_number, driver_id, is_active, created_at"#,
        req.truck_number.trim(),
        req.driver_id
    )
    .fetch_one(&db_pool)
    .await
    .map_err(|e| {
        if let Some(db) = e.as_database_error() {
            if db.code().as_deref() == Some("23505") {
                if db.constraint() == Some("trucks_truck_number_key") {
                    return AppError::conflict("Truck number already exists");
                }
                if db.constraint() == Some("trucks_driver_id_key") {
                    return AppError::conflict("Driver already assigned to another truck");
                }
            }
            if db.code().as_deref() == Some("23503") {
                return AppError::validation("Invalid driver_id");
            }
        }
        AppError::db(e)
    })?;

    // Fetch driver username if assigned
    let driver_username = if let Some(driver_id) = truck.driver_id {
        sqlx::query_scalar!(
            r#"SELECT username FROM users WHERE id = $1"#,
            driver_id
        )
        .fetch_optional(&db_pool)
        .await?
    } else {
        None
    };

    Ok((
        StatusCode::CREATED,
        Json(TruckResponse {
            id: truck.id,
            truck_number: truck.truck_number,
            driver_id: truck.driver_id,
            driver_username,
            is_active: truck.is_active,
            created_at: truck.created_at.unwrap(),
        }),
    ))
}

pub async fn get_truck(
    State(AppState { db_pool }): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<TruckResponse>, AppError> {
    let truck = sqlx::query!(
        r#"SELECT t.id, t.truck_number, t.driver_id, t.is_active, t.created_at, u.username as "driver_username?"
        FROM trucks t
        LEFT JOIN users u ON t.driver_id = u.id
        WHERE t.id = $1"#,
        id
    )
    .fetch_optional(&db_pool)
    .await?
    .ok_or_else(|| AppError::not_found("Truck not found"))?;

    Ok(Json(TruckResponse {
        id: truck.id,
        truck_number: truck.truck_number,
        driver_id: truck.driver_id,
        driver_username: truck.driver_username,
        is_active: truck.is_active,
        created_at: truck.created_at.unwrap(),
    }))
}

pub async fn list_trucks(
    State(AppState { db_pool }): State<AppState>,
) -> Result<Json<Vec<TruckSummary>>, AppError> {
    let trucks = sqlx::query!(
        r#"SELECT t.id, t.truck_number, t.is_active, u.username as "driver_username?"
        FROM trucks t
        LEFT JOIN users u ON t.driver_id = u.id
        ORDER BY t.truck_number ASC"#
    )
    .fetch_all(&db_pool)
    .await?;

    Ok(Json(
        trucks
            .into_iter()
            .map(|t| TruckSummary {
                id: t.id,
                truck_number: t.truck_number,
                driver_username: t.driver_username,
                is_active: t.is_active,
            })
            .collect(),
    ))
}

pub async fn update_truck(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<UpdateTruckRequest>,
) -> Result<Json<TruckResponse>, AppError> {
    if auth.role != "manager" {
        return Err(AppError::forbidden("Only managers can update trucks"));
    }

    // Check if truck exists
    let existing_truck = sqlx::query!("SELECT driver_id FROM trucks WHERE id = $1", id)
        .fetch_optional(&db_pool)
        .await?
        .ok_or_else(|| AppError::not_found("Truck not found"))?;

    let mut truck_number = req.truck_number;
    let mut driver_id = existing_truck.driver_id;
    let mut is_active = None;

    // If driver_id provided, validate it's a driver (not manager)
    if let Some(Some(new_driver_id)) = req.driver_id {
        let driver = sqlx::query!(
            r#"SELECT role FROM users WHERE id = $1"#,
            new_driver_id
        )
        .fetch_optional(&db_pool)
        .await?
        .ok_or_else(|| AppError::not_found("Driver not found"))?;

        if driver.role != "driver" {
            return Err(AppError::validation("Only users with role 'driver' can be assigned to trucks"));
        }
        driver_id = Some(new_driver_id);
    } else if let Some(None) = req.driver_id {
        // Explicitly setting driver_id to None
        driver_id = None;
    }

    if req.is_active.is_some() {
        is_active = req.is_active;
    }

    let truck = sqlx::query!(
        r#"UPDATE trucks SET
            truck_number = COALESCE($2, truck_number),
            driver_id = $3,
            is_active = COALESCE($4, is_active)
        WHERE id = $1
        RETURNING id, truck_number, driver_id, is_active, created_at"#,
        id,
        truck_number.as_deref().map(|s| s.trim()),
        driver_id,
        is_active
    )
    .fetch_one(&db_pool)
    .await
    .map_err(|e| {
        if let Some(db) = e.as_database_error() {
            if db.code().as_deref() == Some("23505") {
                if db.constraint() == Some("trucks_truck_number_key") {
                    return AppError::conflict("Truck number already exists");
                }
                if db.constraint() == Some("trucks_driver_id_key") {
                    return AppError::conflict("Driver already assigned to another truck");
                }
            }
            if db.code().as_deref() == Some("23503") {
                return AppError::validation("Invalid driver_id");
            }
        }
        AppError::db(e)
    })?;

    // Fetch driver username if assigned
    let driver_username = if let Some(driver_id) = truck.driver_id {
        sqlx::query_scalar!(
            r#"SELECT username FROM users WHERE id = $1"#,
            driver_id
        )
        .fetch_optional(&db_pool)
        .await?
    } else {
        None
    };

    Ok(Json(TruckResponse {
        id: truck.id,
        truck_number: truck.truck_number,
        driver_id: truck.driver_id,
        driver_username,
        is_active: truck.is_active,
        created_at: truck.created_at.unwrap(),
    }))
}

pub async fn delete_truck(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<StatusCode, AppError> {
    if auth.role != "manager" {
        return Err(AppError::forbidden("Only managers can delete trucks"));
    }

    // Check if truck has sales
    let has_sales = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM sales WHERE truck_id = $1) as "exists!""#,
        id
    )
    .fetch_one(&db_pool)
    .await?;

    if has_sales {
        return Err(AppError::conflict("Cannot delete truck with existing sales records"));
    }

    // Check if truck has allowances
    let has_allowances = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM truck_allowances WHERE truck_id = $1) as "exists!""#,
        id
    )
    .fetch_one(&db_pool)
    .await?;

    if has_allowances {
        return Err(AppError::conflict("Cannot delete truck with existing allowance records"));
    }

    let result = sqlx::query!("DELETE FROM trucks WHERE id = $1", id)
        .execute(&db_pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("Truck not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}
