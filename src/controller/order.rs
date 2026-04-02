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
use crate::models::order_request::{
    CreateOrderRequest, UpdateStatusRequest, CancelRequest,
};
use crate::models::filter_pagination::PaginationParams;
use crate::services::order as svc;

fn paginated_response(
    message: &str,
    orders: impl serde::Serialize,
    total: i64,
    page: Option<i64>,
    limit: Option<i64>,
) -> serde_json::Value {
    let page  = page.unwrap_or(1);
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
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let order = svc::checkout(&pool, claims.user_id()?, req).await?;

    Ok((StatusCode::CREATED,
        Json(json!({ "success": true,
            "message": "Pesanan berhasil dibuat", "data": order }))))
}

// GET /orders/my/purchases
pub async fn my_purchases(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let page = params.page;
    let limit = params.limit;

    let (orders, total) =
        svc::my_purchases(&pool, claims.user_id()?, params).await?;

    Ok(Json(paginated_response(
        "Riwayat belanja ditemukan", orders, total,
        page, limit,
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

    let (orders, total) =
        svc::my_sales(&pool, claims.user_id()?, params).await?;

    Ok(Json(paginated_response(
        "Daftar pesanan masuk ditemukan", orders, total,
        page, limit,
    )))
}

// GET /orders/{order_id}
pub async fn get_order(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let order = svc::get_order(&pool, order_id, claims.user_id()?).await?;

    Ok(Json(json!({ "success": true, "message": "OK", "data": order })))
}

// GET /orders/{order_id}/history
pub async fn get_order_history(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let history =
        svc::get_order_history(&pool, order_id, claims.user_id()?).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Riwayat ditemukan",
        "data": history
    })))
}

// PATCH /orders/{order_id}/status
pub async fn update_status(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let updated = svc::update_status(
        &pool, order_id, claims.user_id()?, &claims.role, req,
    ).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Status berhasil diperbarui",
        "data": updated
    })))
}

pub async fn confirm_order(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let order = svc::confirm_order(&pool, order_id, claims.user_id()?).await?;

    Ok((StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Pesanan berhasil dikonfirmasi selesai",
            "data": {
                "order_id":     order.order_id,
                "status":       order.status,
                "completed_at": order.updated_at,  // ganti ke completed_at jika field-nya ada
            }
        }))))
}

// POST /orders/{order_id}/cancel
pub async fn cancel_order(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
    Json(req): Json<CancelRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    
    let updated = svc::cancel_order(
        &pool, order_id, claims.user_id()?, &claims.role, req,
    ).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Pesanan berhasil dibatalkan",
        "data": updated
    })))
}
