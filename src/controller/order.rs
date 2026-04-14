use axum::{
    Json,
    extract::{Path, Query, State},
};
use reqwest::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

use crate::error::AppError;
use crate::middleware::auth::JwtClaims;
use crate::models::filter_pagination::PaginationParams;
use crate::models::order::{CancelRequest, CreateOrderRequest, ShippedRequest};
use crate::services::order as svc;

pub fn paginated_response(
    message: &str,
    orders: impl serde::Serialize,
    total: i64,
    page: Option<i64>,
    limit: Option<i64>,
) -> serde_json::Value {
    let page = page.unwrap_or(1);
    let limit = limit.unwrap_or(20);
    json!({
        "success": true, "message": message, "data": orders,
        "pagination": {
            "total_items": total,
            "page": page,
            "limit": limit,
            "total_pages": (total as f64 / limit as f64).ceil() as i64,
        }
    })
}

// POST /orders
pub async fn checkout(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Json(req): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let order = svc::checkout(&pool, claims.user_id()?, req).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({ "success": true,
            "message": "Pesanan berhasil dibuat", "data": order })),
    ))
}

// GET /orders/{order_id}
pub async fn get_order(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let order = svc::get_order(&pool, order_id, claims.user_id()?).await?;

    Ok(Json(
        json!({ "success": true, "message": "OK", "data": order }),
    ))
}

// PATCH /orders/{order_id}/payment
pub async fn payment(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let order_paid = svc::payment(&pool, claims.user_id()?, order_id).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Pembayaran berhasil dilakukan",
        "data": order_paid
    })))
}

// PATCH /orders/{order_id}/confirm
pub async fn confirm_order(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let order = svc::confirm_order(&pool, order_id, claims.user_id()?).await?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Pesanan berhasil dikonfirmasi selesai",
            "data": {
                "order_id":     order.order_id,
                "status":       order.status,
                "completed_at": order.updated_at,
            }
        })),
    ))
}

// PATCH /orders/{order_id}/purchased
pub async fn purchased(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let order = svc::purchased(&pool, order_id, claims.user_id()?).await?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Pesanan berhasil dibeli jastiper",
            "data": {
                "order_id":     order.order_id,
                "status":       order.status,
                "completed_at": order.updated_at,
            }
        })),
    ))
}

// PATCH /orders/{order_id}/shipped
pub async fn shipped(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
    Json(req): Json<ShippedRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let order = svc::shipped(&pool, order_id, claims.user_id()?, req).await?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Pesanan berhasil dikirim jastiper",
            "data": {
                "order_id":     order.order_id,
                "status":       order.status,
                "completed_at": order.updated_at,
            }
        })),
    ))
}

// GET /orders/{order_id}/history
pub async fn get_order_history(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let history = svc::get_order_history(&pool, order_id, claims.user_id()?).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Riwayat ditemukan",
        "data": history
    })))
}

// POST /orders/{order_id}/cancel
pub async fn cancel_order(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
    Json(req): Json<CancelRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let updated =
        svc::cancel_order(&pool, order_id, claims.user_id()?, &claims.role()?, req).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Pesanan berhasil dibatalkan",
        "data": updated
    })))
}

// GET /orders/my/purchases
pub async fn my_purchases(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let page = params.page;
    let limit = params.limit;

    let (orders, total) = svc::my_purchases(&pool, claims.user_id()?, params).await?;

    Ok(Json(paginated_response(
        "Riwayat belanja ditemukan",
        orders,
        total,
        page,
        limit,
    )))
}

// GET /orders/my/sales
pub async fn my_sales(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let page = params.page;
    let limit = params.limit;

    let (orders, total) = svc::my_sales(&pool, claims.user_id()?, params).await?;

    Ok(Json(paginated_response(
        "Daftar pesanan masuk ditemukan",
        orders,
        total,
        page,
        limit,
    )))
}
