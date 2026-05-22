use axum::{
    Json,
    extract::{Path, Query, State},
};
use reqwest::StatusCode;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

use crate::error::AppError;
use crate::middleware::auth::JwtClaims;
use crate::models::filter_pagination::PaginationParams;
use crate::models::rating_product::CreateRatingProductRequest;
use crate::services::rating_product as svc;
use crate::state::AppState;

// POST /orders/{order_id}/rating/product
pub async fn submit_rating_product(
    State(state): State<Arc<AppState>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
    Json(req): Json<CreateRatingProductRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let rating = svc::submit_rating(
        Arc::clone(&state.order_repo),
        Arc::clone(&state.rating_product_repo),
        order_id,
        claims.user_id()?,
        req,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": "Rating berhasil dikirim",
            "data": {
                "rating_id":      rating.rating_product_id,
                "order_id":       rating.order_id,
                "product_rating": rating.product_rating,
                "created_at":     rating.created_at,
            }
        })),
    ))
}

// GET /products/{product_id}/ratings (public — no auth)
pub async fn get_ratings_by_product(
    State(state): State<Arc<AppState>>,
    Path(product_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (ratings, total, average) =
        svc::get_ratings_by_product(Arc::clone(&state.rating_product_repo), product_id, &params)
            .await?;

    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).min(100);

    Ok(Json(json!({
        "success": true,
        "message": "Daftar rating produk ditemukan",
        "data": {
            "ratings": ratings,
            "page": page,
            "limit": limit,
            "total": total,
            "average_rating": average,
        }
    })))
}

// GET /orders/{order_id}/rating/product
pub async fn get_rating(
    State(state): State<Arc<AppState>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rating = svc::get_rating(
        Arc::clone(&state.order_repo),
        Arc::clone(&state.rating_product_repo),
        order_id,
        claims.user_id()?,
    )
    .await?;

    Ok(Json(json!({
        "success": true,
        "message": "Rating ditemukan",
        "data": rating
    })))
}
