use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum AppError {
    // ── 400 ──
    #[error("Validation error: {0}")]
    Validation(String),

    // ── 401 ──
    #[error("{0}")]
    Unauthorized(String),

    // ── 402 Payment Required ──
    #[error("Insufficient balance")]
    InsufficientBalance,

    // ── 403 ──
    #[error("{0}")]
    Forbidden(String),

    // ── 404 ──
    #[error("{0}")]
    NotFound(String),

    // ── 409 Conflict ──
    #[error("{0}")]
    Conflict(String),

    // ── 422 Unprocessable ──
    #[error("{0}")]
    UnprocessableEntity(String),

    // ── 422 Invalid status transition ──
    #[error("Invalid status transition from {current} to {requested}")]
    InvalidStatusTransition {
        current: String,
        requested: String,
        valid: Vec<String>,
    },

    // ── 500 ──
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Limit exceeded: max 1000")]
    LimitExceeded,

    #[error("Internal error")]
    Internal,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            AppError::Validation(m) => (
                StatusCode::BAD_REQUEST,
                json!({"success":false,"message":m,"errors":[{"field":"unknown","message":m}]}),
            ),
            AppError::Unauthorized(m) => (
                StatusCode::UNAUTHORIZED,
                json!({"success":false,"message":m}),
            ),
            AppError::Forbidden(m) => (StatusCode::FORBIDDEN, json!({"success":false,"message":m})),
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, json!({"success":false,"message":m})),
            AppError::Conflict(m) => (StatusCode::CONFLICT, json!({"success":false,"message":m})),
            AppError::UnprocessableEntity(m) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({"success":false,"message":m}),
            ),
            AppError::InvalidStatusTransition {
                current,
                requested,
                valid,
            } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({
                    "success": false,
                    "message": "Invalid status transition",
                    "current_status": current,
                    "requested_status": requested,
                    "valid_transitions": valid,
                }),
            ),
            AppError::Database(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success":false,"message":e.to_string()}),
            ),
            AppError::LimitExceeded => (
                StatusCode::BAD_REQUEST,
                json!({"success":false,"message":"Limit exceeded: max 1000"}),
            ),
            AppError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"success":false,"message":"Internal error"}),
            ),
            AppError::InsufficientBalance => (
                StatusCode::PAYMENT_REQUIRED,
                json!({"success":false,"message":"Insufficient balance"}),
            ),
        };
        (status, Json(body)).into_response()
    }
}
