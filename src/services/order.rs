use sqlx::PgPool;
use uuid::Uuid;
use serde_json::json;
use tracing::{debug, info, warn, error};

use crate::error::AppError;
use crate::models::order::Order;
use crate::models::order_request::{
    CreateOrderRequest, CancelRequest, UpdateStatusRequest,
};
use crate::models::order_status_history::{OrderStatus, OrderStatusHistory};
use crate::models::filter_pagination::{OrderFilter, PaginationParams};
use crate::models::cancelled_by::CancelledBy;
use crate::repositories::{
    order as repo,
    order_status_history as history_repo,
};
use crate::services::inventory_client::{confirm_stock, fetch_product, release_stock, reserve_stock};
use crate::services::wallet_client::{deduct_wallet, refund_wallet};

// ── checkout ──────────────────────────────────────────────────────
pub async fn checkout(
    pool: &PgPool,
    titipers_id: Uuid,
    req: CreateOrderRequest,
) -> Result<Order, AppError> {
    info!("🛒 [checkout] titipers_id={} product_id={} qty={}",
          titipers_id, req.product_id, req.quantity);

    let order_id = Uuid::new_v4();
    debug!("🆔 [checkout] order_id generated={}", order_id);

    let product = fetch_product(req.product_id).await
        .map_err(|e| { error!("❌ [checkout] fetch_product gagal: {:?}", e); e })?;
    debug!("📦 [checkout] product fetched: {}", product);

    let jastiper_id: Uuid =
        serde_json::from_value(product["jastiperId"].clone())
            .map_err(|e| {
                error!("❌ [checkout] parse jastiper_id gagal: {:?}", e);
                AppError::Internal
            })?;
    debug!("👤 [checkout] jastiper_id={}", jastiper_id);

    if titipers_id == jastiper_id {
        warn!("⚠️ [checkout] titipers_id == jastiper_id, forbidden");
        return Err(AppError::Forbidden(
            "Jastiper tidak dapat membeli produk milik sendiri".to_string(),
        ));
    }

    let unit_price  = product["price"].as_i64().unwrap_or(0);
    let service_fee = product["service_fee"].as_i64().unwrap_or(0);
    let total_price = (unit_price + service_fee) * req.quantity as i64;
    debug!("💰 [checkout] unit_price={} service_fee={} total_price={}",
           unit_price, service_fee, total_price);

    let snapshot = json!({
        "product_id":     req.product_id,
        "name":           product["name"],
        "description":    product["description"],
        "image_url":      product["images"][0],
        "origin_country": product["origin_country"],
        "purchase_date":  product["purchase_date"],
        "unit_price":     unit_price,
        "service_fee":    service_fee,
    });
    debug!("📸 [checkout] snapshot={}", snapshot);

    // 1. Reserve stok
    debug!("📦 [checkout] reserving stock product_id={} qty={}", req.product_id, req.quantity);
    reserve_stock(req.product_id, order_id, req.quantity).await
        .map_err(|e| { error!("❌ [checkout] reserve_stock gagal: {:?}", e); e })?;
    info!("✅ [checkout] stock reserved");

    // 2. Deduct wallet — rollback stok jika gagal
    let desc = format!("Pembayaran Order #{}", order_id);
    debug!("💳 [checkout] deducting wallet titipers_id={} amount={}", titipers_id, total_price);
    if let Err(e) = deduct_wallet(titipers_id, order_id, total_price, &desc).await {
        error!("❌ [checkout] deduct_wallet gagal: {:?}, rolling back stock", e);
        let _ = release_stock(req.product_id, order_id, req.quantity).await;
        return Err(e);
    }
    info!("✅ [checkout] wallet deducted");

    // 3. Simpan ke DB — rollback semua jika gagal
    let pid = req.product_id;
    let qty = req.quantity;
    debug!("💾 [checkout] saving order to DB");
    match repo::create(pool, titipers_id, jastiper_id, order_id,
                       req, snapshot, unit_price, service_fee, total_price).await {
        Ok(order) => {
            info!("✅ [checkout] order created successfully order_id={}", order.order_id);
            Ok(order)
        }
        Err(e) => {
            error!("❌ [checkout] repo::create gagal: {:?}, rolling back stock & wallet", e);
            let _ = release_stock(pid, order_id, qty).await;
            let rd = format!("Refund Order #{} - gagal menyimpan", order_id);
            let _ = refund_wallet(titipers_id, order_id, total_price, &rd).await;
            Err(e)
        }
    }
}

// ── get_order ─────────────────────────────────────────────────────
pub async fn get_order(
    pool: &PgPool,
    order_id: Uuid,
    requester_id: Uuid,
) -> Result<Order, AppError> {
    debug!("🔍 [get_order] order_id={} requester_id={}", order_id, requester_id);

    let order = repo::find_by_id(pool, order_id).await
        .map_err(|e| { error!("❌ [get_order] DB error: {:?}", e); e })?
        .ok_or_else(|| {
            warn!("⚠️ [get_order] order not found: {}", order_id);
            AppError::NotFound("Pesanan tidak ditemukan".to_string())
        })?;

    if order.titipers_id != requester_id && order.jastiper_id != requester_id {
        warn!("⚠️ [get_order] forbidden: requester_id={} bukan titipers/jastiper", requester_id);
        return Err(AppError::Forbidden(
            "Anda tidak memiliki akses ke pesanan ini".to_string(),
        ));
    }

    debug!("✅ [get_order] found order_id={} status={:?}", order.order_id, order.status);
    Ok(order)
}

// ── get_order_history ─────────────────────────────────────────────
pub async fn get_order_history(
    pool: &PgPool,
    order_id: Uuid,
    requester_id: Uuid,
) -> Result<Vec<OrderStatusHistory>, AppError> {
    debug!("📜 [get_order_history] order_id={} requester_id={}", order_id, requester_id);

    get_order(pool, order_id, requester_id).await?;

    let history = history_repo::get_status_history(pool, order_id).await
        .map_err(|e| { error!("❌ [get_order_history] DB error: {:?}", e); e })?;

    debug!("✅ [get_order_history] found {} entries", history.len());
    Ok(history)
}

// ── update_status ─────────────────────────────────────────────────
pub async fn update_status(
    pool: &PgPool,
    order_id: Uuid,
    requester_id: Uuid,
    role: &str,
    req: UpdateStatusRequest,
) -> Result<Order, AppError> {
    info!("🔄 [update_status] order_id={} requester_id={} role={} new_status={:?}",
          order_id, requester_id, role, req.status);

    let order = repo::find_by_id(pool, order_id).await
        .map_err(|e| { error!("❌ [update_status] DB error: {:?}", e); e })?
        .ok_or_else(|| {
            warn!("⚠️ [update_status] order not found: {}", order_id);
            AppError::NotFound("Pesanan tidak ditemukan".to_string())
        })?;

    debug!("📋 [update_status] current status={:?}", order.status);

    match (&req.status, role) {
        (OrderStatus::Purchased, "JASTIPER")
        | (OrderStatus::Shipped, "JASTIPER") => {
            if order.jastiper_id != requester_id {
                warn!("⚠️ [update_status] forbidden: bukan jastiper pemilik produk");
                return Err(AppError::Forbidden(
                    "Hanya jastiper pemilik produk".to_string()));
            }
        }
        (OrderStatus::Completed, "TITIPERS") => {
            if order.titipers_id != requester_id {
                warn!("⚠️ [update_status] forbidden: bukan titipers pemilik order");
                return Err(AppError::Forbidden(
                    "Hanya titipers pemilik order".to_string()));
            }
        }
        (_, "ADMIN") => {
            debug!("👑 [update_status] admin override");
        }
        _ => {
            warn!("⚠️ [update_status] role={} tidak punya izin untuk status={:?}", role, req.status);
            return Err(AppError::Forbidden("Role tidak punya izin".to_string()));
        }
    }

    let result = history_repo::update_status(
        pool, order_id, &req.status,
        &requester_id.to_string(), &role.to_uppercase(),
        req.notes.as_deref(),
        req.tracking_number.as_deref(),
        req.courier.as_deref(),
    ).await
        .map_err(|e| { error!("❌ [update_status] DB error: {:?}", e); e })?;

    // 3a. Order selesai → konfirmasi pengurangan stok permanen + update rating
    if req.status == OrderStatus::Completed {
        let product_id: Uuid = serde_json::from_value(
            result.product_snapshot["product_id"].clone()
        ).unwrap_or(result.product_id);

        debug!("✅ [update_status] order Completed, confirming stock product_id={}", product_id);

        // rating dari request jika ada, inventory akan update avg_rating
        let rating = req.rating_product.map(|r| r.product_rating as f64);

        if let Err(e) = confirm_stock(product_id, order_id, rating).await {
            error!("⚠️ [update_status] confirm_stock gagal (non-fatal): {:?}", e);
        } else {
            info!("✅ [update_status] stock confirmed permanently product_id={}", product_id);
        }
    }

    info!("✅ [update_status] order_id={} status updated to {:?}", order_id, req.status);
    Ok(result)
}

// ── cancel_order ──────────────────────────────────────────────────
pub async fn cancel_order(
    pool: &PgPool,
    order_id: Uuid,
    requester_id: Uuid,
    role: &str,
    req: CancelRequest,
) -> Result<Order, AppError> {
    info!("🚫 [cancel_order] order_id={} requester_id={} role={}", order_id, requester_id, role);

    let order = repo::find_by_id(pool, order_id).await
        .map_err(|e| { error!("❌ [cancel_order] DB error: {:?}", e); e })?
        .ok_or_else(|| {
            warn!("⚠️ [cancel_order] order not found: {}", order_id);
            AppError::NotFound("Pesanan tidak ditemukan".to_string())
        })?;

    debug!("📋 [cancel_order] current status={:?}", order.status);

    let (cancelled_by, actor_role) = match role {
        "TITIPERS" => {
            if order.titipers_id != requester_id {
                warn!("⚠️ [cancel_order] forbidden: bukan pemilik order");
                return Err(AppError::Forbidden("Bukan pemilik order".to_string()));
            }
            if order.status != OrderStatus::Paid {
                warn!("⚠️ [cancel_order] invalid status={:?}, harus PAID", order.status);
                return Err(AppError::UnprocessableEntity(
                    "Titipers hanya bisa cancel di status PAID".to_string()));
            }
            (CancelledBy::Titipers, "TITIPERS")
        }
        "JASTIPER" => {
            if order.jastiper_id != requester_id {
                warn!("⚠️ [cancel_order] forbidden: bukan jastiper produk ini");
                return Err(AppError::Forbidden("Bukan jastiper produk ini".to_string()));
            }
            (CancelledBy::Jastiper, "JASTIPER")
        }
        "ADMIN" => {
            debug!("👑 [cancel_order] admin override");
            (CancelledBy::Admin, "ADMIN")
        }
        _ => {
            warn!("⚠️ [cancel_order] role={} tidak dikenali", role);
            return Err(AppError::Forbidden("Role tidak dikenali".to_string()));
        }
    };

    debug!("💾 [cancel_order] saving cancellation to DB");
    let updated = repo::cancel_order(
        pool, order_id, &req.cancellation_reason,
        &cancelled_by, &requester_id.to_string(),
        actor_role, req.notes.as_deref(),
    ).await
        .map_err(|e| { error!("❌ [cancel_order] repo::cancel_order gagal: {:?}", e); e })?;

    let pid: Uuid =
        serde_json::from_value(updated.product_snapshot["product_id"].clone())
            .unwrap_or(updated.product_id);
    debug!("📦 [cancel_order] releasing stock product_id={} qty={}", pid, updated.quantity);
    let _ = release_stock(pid, order_id, updated.quantity).await;

    let rd = format!("Refund Order #{} - dibatalkan", order_id);
    debug!("💳 [cancel_order] refunding wallet titipers_id={} amount={}",
           updated.titipers_id, updated.total_price);
    let _ = refund_wallet(updated.titipers_id, order_id, updated.total_price, &rd).await;

    info!("✅ [cancel_order] order_id={} cancelled successfully", order_id);
    Ok(updated)
}

// ── my_purchases & my_sales ───────────────────────────────────────
pub async fn my_purchases(
    pool: &PgPool,
    titipers_id: Uuid,
    params: PaginationParams,
) -> Result<(Vec<Order>, i64), AppError> {
    debug!("📋 [my_purchases] titipers_id={} page={:?} limit={:?}",
           titipers_id, params.page, params.limit);

    let filter = Some(OrderFilter {
        titipers_id: Some(titipers_id), ..Default::default()
    });

    let result = repo::find_all(pool, filter, params.page, params.limit).await
        .map_err(|e| { error!("❌ [my_purchases] DB error: {:?}", e); e })?;

    debug!("✅ [my_purchases] found {} orders", result.0.len());
    Ok(result)
}

pub async fn my_sales(
    pool: &PgPool,
    jastiper_id: Uuid,
    params: PaginationParams,
) -> Result<(Vec<Order>, i64), AppError> {
    debug!("📋 [my_sales] jastiper_id={} page={:?} limit={:?}",
           jastiper_id, params.page, params.limit);

    let filter = Some(OrderFilter {
        jastiper_id: Some(jastiper_id), ..Default::default()
    });

    let result = repo::find_all(pool, filter, params.page, params.limit).await
        .map_err(|e| { error!("❌ [my_sales] DB error: {:?}", e); e })?;

    debug!("✅ [my_sales] found {} orders", result.0.len());
    Ok(result)
}