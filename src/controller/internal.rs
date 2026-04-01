use axum::{
    Json,
    extract::{Path, State},
};
use reqwest::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use axum::http::HeaderMap;
use uuid::Uuid;
use crate::error::AppError;
use crate::middleware::security_config::validate_service_key;
use crate::models::order_request::PaymentConfirmedRequest;
use crate::services::order_internal as order_internal_svc;

// GET /internal/orders/{order_id}/payment-info
pub async fn payment_info(
    State(pool): State<Arc<PgPool>>,
    Path(order_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    validate_service_key(&headers)?;

    let order = order_internal_svc::get_order_internal(&pool, order_id).await?;

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

// POST /internal/orders/{order_id}/payment-confirmed
pub async fn payment_confirmed(
    State(pool): State<Arc<PgPool>>,
    Path(order_id): Path<Uuid>,
    headers: HeaderMap,                         // ← Headers dulu
    Json(req): Json<PaymentConfirmedRequest>,   // ← Body/Json terakhir
) -> Result<Json<serde_json::Value>, AppError> {
    validate_service_key(&headers)?;

    let order = order_internal_svc::payment_confirmed(&pool, order_id, req).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Status order diperbarui ke PAID",
        "data": {
            "order_id": order.order_id,
            "status":   order.status,
        }
    })))
}