use crate::error::AppError;
use axum::http::HeaderMap;

pub fn validate_service_key(headers: &HeaderMap) -> Result<(), AppError> {
    let service_key = headers
        .get("X-Service-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let expected =
        std::env::var("INTERNAL_SERVICE_KEY").unwrap_or_else(|_| "internal-secret".to_string());

    if service_key != expected {
        return Err(AppError::Unauthorized("Invalid service key".to_string()));
    }

    Ok(())
}
