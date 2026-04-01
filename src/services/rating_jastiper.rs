use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;
use tracing::{debug, info, warn, error};

use crate::error::AppError;
use crate::models::order_status_history::OrderStatus;
use crate::models::rating_jastiper::{CreateRatingJastiperRequest, RatingJastiper};
use crate::repositories::order as order_repo;
use crate::repositories::rating_jastiper as rating_jastiper_repo;
use crate::services::auth_client::send_jastiper_rating;

pub async fn submit_rating_jastiper(
    pool: &PgPool,
    order_id: Uuid,
    titipers_id: Uuid,
    req: CreateRatingJastiperRequest,
) -> Result<RatingJastiper, AppError> {
    info!("⭐ [submit_rating_jastiper] order_id={} titipers_id={}", order_id, titipers_id);

    req.validate().map_err(|e| {
        warn!("⚠️ [submit_rating_jastiper] validasi gagal: {:?}", e);
        AppError::Validation(e.to_string())
    })?;

    let order = order_repo::find_by_id(pool, order_id).await
        .map_err(|e| { error!("❌ [submit_rating_jastiper] DB error: {:?}", e); e })?
        .ok_or_else(|| {
            warn!("⚠️ [submit_rating_jastiper] order tidak ditemukan: {}", order_id);
            AppError::NotFound("Pesanan tidak ditemukan".to_string())
        })?;

    if order.titipers_id != titipers_id {
        warn!("⚠️ [submit_rating_jastiper] forbidden: requester bukan titipers pemilik order");
        return Err(AppError::Forbidden(
            "Anda tidak memiliki akses ke pesanan ini".to_string(),
        ));
    }

    if order.status != OrderStatus::Completed {
        warn!("⚠️ [submit_rating_jastiper] order belum COMPLETED, status={:?}", order.status);
        return Err(AppError::UnprocessableEntity(
            "Rating jastiper hanya dapat diberikan setelah pesanan selesai".to_string(),
        ));
    }

    if rating_jastiper_repo::find_by_order_id(pool, order_id).await?.is_some() {
        warn!("⚠️ [submit_rating_jastiper] rating jastiper sudah ada untuk order_id={}", order_id);
        return Err(AppError::Conflict(
            "Rating jastiper sudah pernah diberikan untuk pesanan ini".to_string(),
        ));
    }

    debug!("💾 [submit_rating_jastiper] saving rating to DB");
    let rating = rating_jastiper_repo::create(pool, order_id, titipers_id, &req).await
        .map_err(|e| { error!("❌ [submit_rating_jastiper] repo::create gagal: {:?}", e); e })?;

    info!("✅ [submit_rating_jastiper] rating jastiper saved rating_id={}", rating.rating_jastiper_id);

    debug!("👤 [submit_rating_jastiper] notifying Profile module jastiper_id={}", order.jastiper_id);
    if let Err(e) = send_jastiper_rating(
        order.jastiper_id,
        order_id,
        req.jastiper_rating,
        req.jastiper_review.as_deref(),
    ).await {
        error!("⚠️ [submit_rating_jastiper] send_jastiper_rating gagal (non-fatal): {:?}", e);
    } else {
        info!("✅ [submit_rating_jastiper] jastiper rating terkirim ke Modul Profil");
    }


    Ok(rating)
}


pub async fn get_rating(
    pool: &PgPool,
    order_id: Uuid,
    requester_id: Uuid,
) -> Result<RatingJastiper, AppError> {
    debug!("🔍 [get_rating] order_id={} requester_id={}", order_id, requester_id);

    let order = order_repo::find_by_id(pool, order_id).await?
        .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    if order.titipers_id != requester_id && order.jastiper_id != requester_id {
        warn!("⚠️ [get_rating] forbidden: requester_id={} bukan titipers/jastiper", requester_id);
        return Err(AppError::Forbidden(
            "Anda tidak memiliki akses ke pesanan ini".to_string(),
        ));
    }

    rating_jastiper_repo::find_by_order_id(pool, order_id).await?
        .ok_or_else(|| {
            warn!("⚠️ [get_rating] rating jastiper belum ada untuk order_id={}", order_id);
            AppError::NotFound("Rating jastiper belum ada untuk pesanan ini".to_string())
        })
}