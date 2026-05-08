use crate::controller::order::paginated_response;
use crate::error::AppError;
use crate::middleware::auth::JwtClaims;
use crate::models::filter_pagination::OrderQueryParams;
use crate::models::order::CancelRequest;
use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, Query, State};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

// GET /admin/orders
pub async fn get_all(
    State(state): State<Arc<AppState>>,
    claims: JwtClaims,
    Query(params): Query<OrderQueryParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (orders, total) = crate::services::admin::get_all(
        Arc::clone(&state.order_repo),
        &params.filter,
        &params.pagination,
        &claims.role()?,
    )
    .await?;

    Ok(Json(paginated_response(
        "Riwayat belanja ditemukan",
        orders,
        total,
        Option::from(params.pagination.page.unwrap_or(1)),
        Option::from(params.pagination.limit.unwrap_or(20)),
    )))
}

// GET /admin/orders/{order_id}
pub async fn get_order(
    State(state): State<Arc<AppState>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let order =
        crate::services::admin::get_order(Arc::clone(&state.order_repo), order_id, &claims.role()?)
            .await?;

    Ok(Json(json!({
        "success": true,
        "message": "OK",
        "data": order,
    })))
}

// POST  /admin/orders/{order_id}/force-cancel
pub async fn force_cancel(
    State(state): State<Arc<AppState>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
    Json(req): Json<CancelRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let order = crate::services::admin::force_cancel(
        Arc::clone(&state.order_repo),
        Arc::clone(&state.inventory_client),
        Arc::clone(&state.wallet_client),
        order_id,
        claims.user_id()?,
        &claims.role()?,
        req,
    )
    .await?;

    Ok(Json(json!({
        "success": true,
        "message": "Pesanan berhasil dibatalkan",
        "data": order,
    })))
}
