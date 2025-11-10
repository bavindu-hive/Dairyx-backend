use chrono::{Utc, Duration};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, Algorithm};
use serde::{Serialize, Deserialize};
use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
    pub username: String,
}

pub fn sign_token(user_id: i64, role: &str, username: &str, secret: &str) -> Result<String, AppError> {
    let now = Utc::now();
    let exp = now + Duration::hours(8);
    let claims = Claims {
        sub: user_id,
        role: role.to_string(),
        iat: now.timestamp() as usize,
        exp: exp.timestamp() as usize,
        username: username.to_string(),
    };
    encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|e| AppError::internal(format!("Token signing failed: {e}")))
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256)
    )
    .map(|d| d.claims)
    .map_err(|e| AppError::validation(format!("Invalid or expired token: {e}")))
}