use crate::error::AppError;
use crate::models::order::{OrderFilter, PaginationParams};
use crate::repositories::order as repo;
use axum::Json;
use axum::extract::{Path, Query, State};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use utoipa::ToSchema;

// ── GET /orders/{order_id} — Detail ──────────────────────────────
#[utoipa::path(
    get, path = "/orders/{order_id}",
    tag = "Orders",
    params(("order_id" = Uuid, Path, description = "ID unik pesanan")),
    responses(
        (status=200, description="Data pesanan ditemukan", body=Order),
        (status=403, description="Bukan pesanan milik user ini"),
        (status=404, description="Pesanan tidak ditemukan"),
    )
)]
pub async fn get_order(
    State(pool): State<Arc<PgPool>>,
    Path(order_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let order = repo::find_by_id(&pool, order_id).await?;
    Ok(Json(json!({"success":true,"message":"OK","data":order})))
}

// --- GET /orders/my/purchases ---
#[utoipa::path(
    get,
    path = "/orders/my/purchases",
    tag = "Orders",
    params(
        ("page" = Option<i64>, Query, description = "Halaman (default: 1)"),
        ("limit" = Option<i64>, Query, description = "Item per halaman (default: 20)")
    ),
    responses(
        (status = 200, description = "Berhasil", body = serde_json::Value),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn my_purchases(
    State(pool): State<Arc<PgPool>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let titipers_id = Uuid::new_v4(); // Nanti ganti dengan ID dari JWT

    let filter = Some(OrderFilter {
        titipers_id: Some(titipers_id),
        ..Default::default()
    });

    let (orders, total_count) = repo::find_all(&pool, filter, params.page, params.limit).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Riwayat belanja ditemukan",
        "data": orders,
        "pagination": {
            "total_items": total_count,
            "page": params.page.unwrap_or(1),
            "limit": params.limit.unwrap_or(20),
            "total_pages": (total_count as f64 / params.limit.unwrap_or(20) as f64).ceil() as i64
        }
    })))
}

// --- GET /orders/my/sales ---
#[utoipa::path(
    get,
    path = "/orders/my/sales",
    tag = "Orders",
    params(
        ("page" = Option<i64>, Query, description = "Halaman"),
        ("limit" = Option<i64>, Query, description = "Limit")
    ),
    responses(
        (status = 200, description = "Berhasil", body = serde_json::Value),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn my_sales(
    State(pool): State<Arc<PgPool>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let jastiper_id = Uuid::new_v4();

    let filter = Some(OrderFilter {
        jastiper_id: Some(jastiper_id),
        ..Default::default()
    });

    let (orders, total_count) = repo::find_all(&pool, filter, params.page, params.limit).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Daftar pesanan masuk ditemukan",
        "data": orders,
        "pagination": {
            "total_items": total_count,
            "page": params.page.unwrap_or(1),
            "limit": params.limit.unwrap_or(20)
        }
    })))
}
