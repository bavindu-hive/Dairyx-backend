use axum::{extract::State, Json, Extension};
use axum::http::StatusCode;
use crate::state::AppState;
use crate::error::AppError;
use crate::dtos::shop::{CreateShopRequest, UpdateShopRequest, ShopResponse, ShopSummary};
use crate::middleware::auth::AuthContext;

pub async fn create_shop(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateShopRequest>,
) -> Result<(StatusCode, Json<ShopResponse>), AppError> {
    if auth.role != "manager" {
        return Err(AppError::forbidden("Only managers can create shops"));
    }

    if req.name.trim().is_empty() {
        return Err(AppError::validation("Shop name is required"));
    }

    // Validate distance is not negative
    if let Some(dist) = req.distance {
        if dist < 0.0 {
            return Err(AppError::validation("Distance cannot be negative"));
        }
    }

    let shop = sqlx::query!(
        r#"INSERT INTO shops (name, location, contact_info, distance)
        VALUES ($1, $2, $3, $4::FLOAT8)
        RETURNING id, name, location, contact_info, (distance)::FLOAT8 as "distance?", created_at"#,
        req.name.trim(),
        req.location,
        req.contact_info,
        req.distance
    )
    .fetch_one(&db_pool)
    .await
    .map_err(|e| {
        if let Some(db) = e.as_database_error() {
            if db.code().as_deref() == Some("23505") {
                return AppError::conflict("Shop name already exists");
            }
        }
        AppError::db(e)
    })?;

    Ok((
        StatusCode::CREATED,
        Json(ShopResponse {
            id: shop.id,
            name: shop.name,
            location: shop.location,
            contact_info: shop.contact_info,
            distance: shop.distance,
            created_at: shop.created_at.unwrap(),
        }),
    ))
}

pub async fn get_shop(
    State(AppState { db_pool }): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<ShopResponse>, AppError> {
    let shop = sqlx::query!(
        r#"SELECT id, name, location, contact_info, (distance)::FLOAT8 as "distance?", created_at
        FROM shops
        WHERE id = $1"#,
        id
    )
    .fetch_optional(&db_pool)
    .await?
    .ok_or_else(|| AppError::not_found("Shop not found"))?;

    Ok(Json(ShopResponse {
        id: shop.id,
        name: shop.name,
        location: shop.location,
        contact_info: shop.contact_info,
        distance: shop.distance,
        created_at: shop.created_at.unwrap(),
    }))
}

pub async fn list_shops(
    State(AppState { db_pool }): State<AppState>,
) -> Result<Json<Vec<ShopSummary>>, AppError> {
    let shops = sqlx::query!(
        r#"SELECT id, name, location, (distance)::FLOAT8 as "distance?"
        FROM shops
        ORDER BY name ASC"#
    )
    .fetch_all(&db_pool)
    .await?;

    Ok(Json(
        shops
            .into_iter()
            .map(|s| ShopSummary {
                id: s.id,
                name: s.name,
                location: s.location,
                distance: s.distance,
            })
            .collect(),
    ))
}

pub async fn update_shop(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<UpdateShopRequest>,
) -> Result<Json<ShopResponse>, AppError> {
    if auth.role != "manager" {
        return Err(AppError::forbidden("Only managers can update shops"));
    }

    // Validate distance is not negative
    if let Some(dist) = req.distance {
        if dist < 0.0 {
            return Err(AppError::validation("Distance cannot be negative"));
        }
    }

    // Check if shop exists
    let _existing = sqlx::query!("SELECT id FROM shops WHERE id = $1", id)
        .fetch_optional(&db_pool)
        .await?
        .ok_or_else(|| AppError::not_found("Shop not found"))?;

    let shop = sqlx::query!(
        r#"UPDATE shops SET
            name = COALESCE($2, name),
            location = COALESCE($3, location),
            contact_info = COALESCE($4, contact_info),
            distance = COALESCE($5::FLOAT8, distance)
        WHERE id = $1
        RETURNING id, name, location, contact_info, (distance)::FLOAT8 as "distance?", created_at"#,
        id,
        req.name.as_deref().map(|s| s.trim()),
        req.location,
        req.contact_info,
        req.distance
    )
    .fetch_one(&db_pool)
    .await
    .map_err(|e| {
        if let Some(db) = e.as_database_error() {
            if db.code().as_deref() == Some("23505") {
                return AppError::conflict("Shop name already exists");
            }
        }
        AppError::db(e)
    })?;

    Ok(Json(ShopResponse {
        id: shop.id,
        name: shop.name,
        location: shop.location,
        contact_info: shop.contact_info,
        distance: shop.distance,
        created_at: shop.created_at.unwrap(),
    }))
}

pub async fn delete_shop(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<StatusCode, AppError> {
    if auth.role != "manager" {
        return Err(AppError::forbidden("Only managers can delete shops"));
    }

    // Check if shop has sales
    let has_sales = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM sales WHERE shop_id = $1) as "exists!""#,
        id
    )
    .fetch_one(&db_pool)
    .await?;

    if has_sales {
        return Err(AppError::conflict("Cannot delete shop with existing sales records"));
    }

    let result = sqlx::query!("DELETE FROM shops WHERE id = $1", id)
        .execute(&db_pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("Shop not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}
