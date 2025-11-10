use bcrypt::{hash, verify, DEFAULT_COST};
use crate::dtos::user::{RegisterUserRequest, UserResponse, LoginRequest, LoginResponse};
use crate::auth::jwt::sign_token;
use crate::error::AppError;
use axum::{extract::State, Json};
use crate::state::AppState;
use crate::middleware::auth::AuthContext;
use axum::extract::Extension;


pub async fn register_user(
    State(AppState { db_pool }): State<AppState>,
    Json(payload): Json<RegisterUserRequest>
) -> Result<(axum::http::StatusCode, Json<UserResponse>), AppError> {
    // Basic validation
    if payload.role != "manager" && payload.role != "driver" {
        return Err(AppError::validation("Invalid role"));
    }
    if payload.username.trim().is_empty() {
        return Err(AppError::validation("Username required"));
    }
    if payload.password.len() < 6 {
        return Err(AppError::validation("Password too short"));
    }

    let password_hash = hash(&payload.password, DEFAULT_COST)
        .map_err(|e| AppError::internal(format!("Hash error: {e}")))?;

    let rec = sqlx::query_as!(
        UserInsertReturn,
        r#"
        INSERT INTO users (username, password_hash, role)
        VALUES ($1, $2, $3)
    RETURNING id, username, role, is_active, created_at as "created_at!"
        "#,
        payload.username,
        password_hash,
        payload.role
    )
    .fetch_one(&db_pool)
    .await
    .map_err(|e| {
        if let Some(db_err) = e.as_database_error() {
            if db_err.code().as_deref() == Some("23505") {
                return AppError::conflict("Username already exists");
            }
        }
        AppError::db(e)
    })?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(UserResponse {
            id: rec.id,
            username: rec.username,
            role: rec.role,
            is_active: rec.is_active,
            created_at: rec.created_at,
        }),
    ))
}

pub async fn login_user(
    State(AppState { db_pool }): State<AppState>,
    Json(payload): Json<LoginRequest>
) -> Result<Json<LoginResponse>, AppError> {
    if payload.username.trim().is_empty() {
        return Err(AppError::validation("Username required"));
    }
    if payload.password.is_empty() {
        return Err(AppError::validation("Password required"));
    }

    let user = sqlx::query_as!(
        UserRow,
        r#"SELECT id, username, password_hash, role, is_active FROM users WHERE username = $1"#,
        payload.username
    )
    .fetch_optional(&db_pool)
    .await?
    .ok_or_else(|| AppError::not_found("Invalid credentials"))?;

    if !user.is_active {
        return Err(AppError::conflict("User inactive"));
    }

    let ok = verify(&payload.password, &user.password_hash)
        .map_err(|e| AppError::internal(format!("Password verify error: {e}")))?;

    if !ok {
        return Err(AppError::validation("Invalid credentials"));
    }

    let secret = std::env::var("JWT_SECRET")
        .map_err(|_| AppError::internal("JWT secret not configured"))?;

    let token = sign_token(user.id, &user.role, &user.username, &secret)?;

    // 8 hours = 28800 seconds
    Ok(Json(LoginResponse {
        access_token: token,
        token_type: "Bearer",
        expires_in_seconds: 8 * 60 * 60,
    }))
}

// Authenticated endpoint: returns full user profile from DB using the id in AuthContext
pub async fn get_me(
    State(AppState { db_pool }): State<AppState>,
    Extension(auth): Extension<AuthContext>
) -> Result<Json<UserResponse>, AppError> {
    let rec = sqlx::query_as!(
        UserProfileRow,
        r#"SELECT id, username, role, is_active, created_at as "created_at!" FROM users WHERE id = $1"#,
        auth.user_id
    )
    .fetch_one(&db_pool)
    .await?;

    Ok(Json(UserResponse {
        id: rec.id,
        username: rec.username,
        role: rec.role,
        is_active: rec.is_active,
        created_at: rec.created_at,
    }))
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: i64,
    username: String,
    password_hash: String,
    role: String,
    is_active: bool,
}

struct UserInsertReturn {
    id: i64,
    username: String,
    role: String,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

struct UserProfileRow {
    id: i64,
    username: String,
    role: String,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}