use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;
use tracing::{debug, info, warn, error};

use crate::error::AppError;
use crate::models::order_status_history::OrderStatus;
use crate::models::rating_product::{CreateRatingProductRequest, RatingProduct};
use crate::repositories::order as order_repo;
use crate::repositories::rating_product as rating_product_repo;
use crate::services::inventory_client::send_product_rating;

pub async fn submit_rating(
    pool: &PgPool,
    order_id: Uuid,
    titipers_id: Uuid,
    req: CreateRatingProductRequest,
) -> Result<RatingProduct, AppError> {
    info!("⭐ [submit_rating_product] order_id={} titipers_id={}", order_id, titipers_id);

    req.validate().map_err(|e| {
        warn!("⚠️ [submit_rating_product] validasi gagal: {:?}", e);
        AppError::Validation(e.to_string())
    })?;

    let order = order_repo::find_by_id(pool, order_id).await
        .map_err(|e| { error!("❌ [submit_rating_product] DB error: {:?}", e); e })?
        .ok_or_else(|| {
            warn!("⚠️ [submit_rating_product] order tidak ditemukan: {}", order_id);
            AppError::NotFound("Pesanan tidak ditemukan".to_string())
        })?;

    if order.titipers_id != titipers_id {
        warn!("⚠️ [submit_rating_product] forbidden: requester bukan titipers pemilik order");
        return Err(AppError::Forbidden(
            "Anda tidak memiliki akses ke pesanan ini".to_string(),
        ));
    }

    if order.status != OrderStatus::Completed {
        warn!("⚠️ [submit_rating_product] order belum COMPLETED, status={:?}", order.status);
        return Err(AppError::UnprocessableEntity(
            "Rating product hanya dapat diberikan setelah pesanan selesai".to_string(),
        ));
    }

    if rating_product_repo::find_by_order_id(pool, order_id).await?.is_some() {
        warn!("⚠️ [submit_rating_product] rating product sudah ada untuk order_id={}", order_id);
        return Err(AppError::Conflict(
            "Rating sudah pernah diberikan untuk pesanan ini".to_string(),
        ));
    }

    debug!("💾 [submit_rating_product] saving rating to DB");
    let rating = rating_product_repo::create(pool, order_id, titipers_id, &req).await
        .map_err(|e| { error!("❌ [submit_rating_product] repo::create gagal: {:?}", e); e })?;

    info!("✅ [submit_rating_product] rating saved rating_id={}", rating.rating_product_id);

    let product_id: Uuid = serde_json::from_value(
        order.product_snapshot["product_id"].clone()
    ).unwrap_or(order.product_id);

    let images = req.product_images.clone().unwrap_or_default();

    debug!("📦 [submit_rating_product] notifying Inventory module product_id={}", product_id);
    if let Err(e) = send_product_rating(
        product_id,
        order_id,
        req.product_rating,
        req.product_review.as_deref(),
        &images,
    ).await {
        error!("⚠️ [submit_rating_product] send_product_rating gagal (non-fatal): {:?}", e);
    } else {
        info!("✅ [submit_rating_product] product rating terkirim ke Modul Inventory");
    }

    Ok(rating)
}

pub async fn get_rating(
    pool: &PgPool,
    order_id: Uuid,
    requester_id: Uuid,
) -> Result<RatingProduct, AppError> {
    debug!("🔍 [get_rating] order_id={} requester_id={}", order_id, requester_id);

    let order = order_repo::find_by_id(pool, order_id).await?
        .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    if order.titipers_id != requester_id && order.jastiper_id != requester_id {
        warn!("⚠️ [get_rating] forbidden: requester_id={} bukan titipers/jastiper", requester_id);
        return Err(AppError::Forbidden(
            "Anda tidak memiliki akses ke pesanan ini".to_string(),
        ));
    }

    rating_product_repo::find_by_order_id(pool, order_id).await?
        .ok_or_else(|| {
            warn!("⚠️ [get_rating] rating belum ada untuk order_id={}", order_id);
            AppError::NotFound("Rating belum ada untuk pesanan ini".to_string())
        })
}