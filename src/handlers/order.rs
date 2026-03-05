use std::sync::Arc;
use axum::extract::{Query, State};
use axum::Json;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use crate::error::AppError;
use crate::models::order::{OrderFilter, PaginationParams};
use crate::repositories::order as repo;

use utoipa::ToSchema;

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