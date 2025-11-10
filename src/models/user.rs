use serde::Serialize;
use chrono::{DateTime, Utc};

#[derive(sqlx::FromRow, Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}