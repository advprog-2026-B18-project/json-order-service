use axum::{
    Json,
    extract::{Path, State},
};
use reqwest::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;
use crate::error::AppError;
use crate::middleware::auth::JwtClaims;
use crate::services::order as order_svc;

// GET /internal/orders/{order_id}/payment-info
pub async fn payment_info(
    State(pool): State<Arc<PgPool>>,
    Path(order_id): Path<Uuid>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let service_key = headers
        .get("X-Service-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let expected = std::env::var("INTERNAL_SERVICE_KEY")
        .unwrap_or_else(|_| "internal-secret".to_string());

    if service_key != expected {
        return Err(AppError::Unauthorized("Invalid service key".to_string()));
    }

    let order = order_svc::get_order_internal(&pool, order_id).await?;

    Ok(Json(json!({
        "success": true,
        "message": "OK",
        "data": {
            "order_id":        order.order_id,
            "titipers_user_id": order.titipers_id,
            "jastiper_user_id": order.jastiper_id,
            "total_price":     order.total_price,
            "status":          order.status,
            "product_snapshot": order.product_snapshot,
        }
    })))
}