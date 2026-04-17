use crate::error::AppError;
use crate::middleware::security_config::validate_service_key;
use crate::models::order::{PaymentConfirmedRequest, RefundConfirmedRequest};
use crate::services::order_internal as order_internal_svc;
use crate::state::AppState;
use axum::http::HeaderMap;
use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

// GET /internal/orders/{order_id}/payment-info
pub async fn payment_info(
    State(state): State<Arc<AppState>>,
    Path(order_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    validate_service_key(&headers)?;

    let order = order_internal_svc::get_order_internal(state.order_repo.as_ref(), order_id).await?;

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
    State(state): State<Arc<AppState>>,
    Path(order_id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<PaymentConfirmedRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    validate_service_key(&headers)?;

    let order =
        order_internal_svc::payment_confirmed(state.order_repo.as_ref(), order_id, req).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Status order diperbarui ke PAID",
        "data": {
            "order_id": order.order_id,
            "status":   order.status,
        }
    })))
}

// POST /internal/orders/{order_id}/refund-confirmed
pub async fn refund_confirmed(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(order_id): Path<Uuid>,
    Json(req): Json<RefundConfirmedRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    validate_service_key(&headers)?;

    let order =
        order_internal_svc::refund_confirmed(state.order_repo.as_ref(), order_id, req).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Refund terkonfirmasi",
        "data": {
            "order_id":        order.order_id,
            "status":          order.status,
            "refund_confirmed": true,
        }
    })))
}
