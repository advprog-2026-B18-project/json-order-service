use axum::{extract::FromRequestParts, http::request::Parts};
use base64::{Engine, engine::general_purpose};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::warn;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::role::Role;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    pub sub: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub role: String,
    pub exp: usize,
    #[serde(default)]
    pub iat: usize,
}

impl JwtClaims {
    pub fn user_id(&self) -> Result<Uuid, AppError> {
        Uuid::parse_str(&self.sub)
            .map_err(|_| AppError::Unauthorized("Token subject bukan UUID valid".to_string()))
    }

    pub fn role(&self) -> Result<Role, AppError> {
        Role::from_str(&self.role)
            .map_err(|e| AppError::Unauthorized(format!("Role tidak valid: {}", e)))
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

        let secret_clean: String = secret.chars().filter(|c| !c.is_whitespace()).collect();

        let padded = match secret_clean.len() % 4 {
            2 => format!("{}==", secret_clean),
            3 => format!("{}=", secret_clean),
            _ => secret_clean.clone(),
        };

        let padded = if padded.ends_with("==") {
            let mut chars: Vec<char> = padded.chars().collect();
            let last_data_idx = padded.len() - 3;
            let last_char = chars[last_data_idx];
            let b64_chars: Vec<char> =
                "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
                    .chars()
                    .collect();
            let idx = b64_chars.iter().position(|&c| c == last_char).unwrap_or(0);
            chars[last_data_idx] = b64_chars[idx & !0x3];
            chars.iter().collect()
        } else {
            padded
        };

        let decoded_secret = general_purpose::STANDARD.decode(&padded).map_err(|e| {
            warn!("❌ Base64 decode gagal: {:?}", e);
            AppError::Unauthorized("JWT_SECRET tidak valid".to_string())
        })?;

        let decoding_key = DecodingKey::from_secret(&decoded_secret);

        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        let token_data = decode::<JwtClaims>(token, &decoding_key, &validation).map_err(|e| {
            warn!("❌ Token tidak valid: {:?}", e);
            AppError::Unauthorized(format!("Token tidak valid: {}", e))
        })?;

        Ok(token_data.claims)
    }
}
