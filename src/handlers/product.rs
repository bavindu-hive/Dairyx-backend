// src/handlers/products.rs
use axum::{
    extract::{Path, State},
    Json,
};
use sqlx::Error as SqlxError;
use crate::dtos::product::{CreateProductRequest, UpdateProductRequest, ProductResponse};
use crate::models::product::Product;
use crate::state::AppState;
use crate::error::AppError;
use tracing::{error, instrument};

fn map_unique_violation(err: SqlxError, message: &str) -> AppError {
    match err {
        SqlxError::Database(db_err) if db_err.code().as_deref() == Some("23505") => {
            AppError::conflict(message)
        }
        other => other.into(),
    }
}

// GET /products - List all products
#[instrument(skip(state))]
pub async fn get_products(State(state): State<AppState>) -> Result<Json<Vec<ProductResponse>>, AppError> {
    match sqlx::query_as::<_, Product>(
        "SELECT id, name,
                current_wholesale_price::FLOAT8 AS current_wholesale_price,
                commission_per_unit::FLOAT8     AS commission_per_unit,
                created_at
         FROM products ORDER BY name"
    )
        .fetch_all(&state.db_pool)
        .await {
        Ok(products) => {
            let response = products.into_iter().map(ProductResponse::from).collect();
            Ok(Json(response))
        }
        Err(e) => {
            error!(?e, "Failed to fetch products");
            Err(e.into())
        }
    }
}

// GET /products/:id - Get single product
#[instrument(skip(state), fields(id))]
pub async fn get_product(
    Path(id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Json<ProductResponse>, AppError> {
    let product = sqlx::query_as::<_, Product>(
        "SELECT id, name,
                current_wholesale_price::FLOAT8 AS current_wholesale_price,
                commission_per_unit::FLOAT8     AS commission_per_unit,
                created_at
         FROM products WHERE id = $1"
    )
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await?
    .ok_or_else(|| AppError::not_found("Product not found"))?;

    Ok(Json(ProductResponse::from(product)))
}

// POST /products - Create new product
#[instrument(skip(state, payload))]
pub async fn create_product(
    State(state): State<AppState>,
    Json(payload): Json<CreateProductRequest>,
) -> Result<Json<ProductResponse>, AppError> {
    let product = sqlx::query_as::<_, Product>(
        "INSERT INTO products (name, current_wholesale_price, commission_per_unit) 
         VALUES ($1, $2, $3) RETURNING id, name,
                current_wholesale_price::FLOAT8 AS current_wholesale_price,
                commission_per_unit::FLOAT8     AS commission_per_unit,
                created_at"
    )
    .bind(&payload.name)
    .bind(payload.current_wholesale_price)
    .bind(payload.commission_per_unit)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| map_unique_violation(e, "Product name already exists"))?;

    Ok(Json(ProductResponse::from(product)))
}

// PUT /products/:id - Update product
#[instrument(skip(state, payload), fields(id))]
pub async fn update_product(
    Path(id): Path<i64>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateProductRequest>,
) -> Result<Json<ProductResponse>, AppError> {
    let product = sqlx::query_as::<_, Product>(
        "UPDATE products SET 
         name = COALESCE($1, name),
         current_wholesale_price = COALESCE($2, current_wholesale_price),
         commission_per_unit = COALESCE($3, commission_per_unit)
         WHERE id = $4 RETURNING id, name,
                current_wholesale_price::FLOAT8 AS current_wholesale_price,
                commission_per_unit::FLOAT8     AS commission_per_unit,
                created_at"
    )
    .bind(payload.name)
    .bind(payload.current_wholesale_price)
    .bind(payload.commission_per_unit)
    .bind(id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| map_unique_violation(e, "Product name already exists"))?
    .ok_or_else(|| AppError::not_found("Product not found"))?;

    Ok(Json(ProductResponse::from(product)))
}

// DELETE /products/:id - Delete product
#[instrument(skip(state), fields(id))]
pub async fn delete_product(
    Path(id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Json<()>, AppError> {
    let result = sqlx::query("DELETE FROM products WHERE id = $1")
        .bind(id)
        .execute(&state.db_pool)
        .await?;

    if result.rows_affected() == 0 {
    return Err(AppError::not_found("Product not found"));
    }

    Ok(Json(()))
}