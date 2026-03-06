use axum::{
    extract::FromRequestParts,
    http::{request::Parts},
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    pub sub: String,
    pub email: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

impl JwtClaims {
    pub fn user_id(&self) -> Result<Uuid, AppError> {
        Uuid::parse_str(&self.sub)
            .map_err(|_| AppError::Unauthorized("Token subject bukan UUID valid".to_string()))
    }
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for JwtClaims
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                AppError::Unauthorized("Header Authorization tidak ditemukan".to_string())
            })?;

        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            AppError::Unauthorized("Format token harus 'Bearer <token>'".to_string())
        })?;

        let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "change-me".to_string());

        let decoding_key = DecodingKey::from_secret(secret.as_bytes());

        let mut validation = Validation::new(Algorithm::HS256);

        validation.validate_exp = true;

        let token_data = decode::<JwtClaims>(token, &decoding_key, &validation)
            .map_err(|e| AppError::Unauthorized(format!("Token tidak valid: {}", e)))?;

        Ok(token_data.claims)
    }
}
