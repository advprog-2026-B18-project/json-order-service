use serde_json::json;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::filter_pagination::{OrderFilter, PaginationParams};
use crate::models::order::{
    CancelRequest, CreateOrderRequest, Order, PriceBreakdown, ShippedRequest, UpdateOrderParams,
    UpdateStatusRequest,
};
use crate::models::order_state::OrderMachine;
use crate::models::order_status_history::{OrderStatus, OrderStatusHistory};
use crate::models::role::Role;
use crate::ports::inventory_client::InventoryClient;
use crate::ports::order_repository::OrderRepository;
use crate::ports::order_status_history_repository::OrderStatusHistoryRepository;
use crate::ports::wallet_client::WalletClient;

// ── checkout ──────────────────────────────────────────────────────
pub async fn checkout(
    order_repo: &dyn OrderRepository,
    inventory_client: &dyn InventoryClient,
    wallet_client: &dyn WalletClient,
    titipers_id: Uuid,
    req: CreateOrderRequest,
) -> Result<Order, AppError> {
    info!(
        "🛒 [checkout] titipers_id={} product_id={} qty={}",
        titipers_id, req.product_id, req.quantity
    );

    let order_id = Uuid::new_v4();

    let product = inventory_client
        .fetch_product(req.product_id)
        .await
        .map_err(|e| {
            error!("❌ [checkout] fetch_product gagal: {:?}", e);
            e
        })?;

    let jastiper_id: Uuid =
        serde_json::from_value(product["jastiperId"].clone()).map_err(|_| AppError::Internal)?;

    if titipers_id == jastiper_id {
        return Err(AppError::Forbidden(
            "Jastiper tidak dapat membeli produk milik sendiri".to_string(),
        ));
    }

    let unit_price = product["price"].as_i64().unwrap_or(0);
    let service_fee = product["service_fee"].as_i64().unwrap_or(0);
    let total_price = (unit_price + service_fee) * req.quantity as i64;

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

    inventory_client
        .reserve_stock(req.product_id, order_id, req.quantity)
        .await
        .map_err(|e| {
            error!("❌ [checkout] reserve_stock gagal: {:?}", e);
            e
        })?;

    if let Err(e) = wallet_client.check_wallet(titipers_id, total_price).await {
        error!("❌ [checkout] check_wallet gagal, releasing stock: {:?}", e);
        let _ = inventory_client
            .release_stock(req.product_id, order_id, req.quantity)
            .await;
        return Err(e);
    }

    let pid = req.product_id;
    let qty = req.quantity;

    match order_repo
        .create(
            titipers_id,
            jastiper_id,
            req,
            snapshot,
            PriceBreakdown {
                unit_price,
                service_fee,
                total_price,
            },
        )
        .await
    {
        Ok(order) => {
            info!("✅ [checkout] order created order_id={}", order.order_id);
            Ok(order)
        }
        Err(e) => {
            error!("❌ [checkout] order create gagal, releasing stock: {:?}", e);
            let _ = inventory_client.release_stock(pid, order_id, qty).await;
            Err(e)
        }
    }
}

// ── get_order ─────────────────────────────────────────────────────
pub async fn get_order(
    order_repo: &dyn OrderRepository,
    order_id: Uuid,
    requester_id: Uuid,
) -> Result<Order, AppError> {
    debug!(
        "🔍 [get_order] order_id={} requester_id={}",
        order_id, requester_id
    );

    let order = order_repo
        .find_by_id(order_id)
        .await
        .map_err(|e| {
            error!("❌ [get_order] DB error: {:?}", e);
            e
        })?
        .ok_or_else(|| {
            warn!("⚠️ [get_order] order not found: {}", order_id);
            AppError::NotFound("Pesanan tidak ditemukan".to_string())
        })?;

    if order.titipers_id != requester_id && order.jastiper_id != requester_id {
        warn!(
            "⚠️ [get_order] forbidden: requester_id={} bukan titipers/jastiper",
            requester_id
        );
        return Err(AppError::Forbidden(
            "Anda tidak memiliki akses ke pesanan ini".to_string(),
        ));
    }

    debug!(
        "✅ [get_order] found order_id={} status={:?}",
        order.order_id, order.status
    );
    Ok(order)
}

// ── update_status ─────────────────────────────────────────────────
pub async fn update_status(
    order_repo: &dyn OrderRepository,
    order_id: Uuid,
    requester_id: Uuid,
    role: &Role,
    req: UpdateStatusRequest,
) -> Result<Order, AppError> {
    info!(
        "🔄 [update_order] order_id={} requester_id={} role={} new_status={:?}",
        order_id, requester_id, role, req.status
    );

    let order = order_repo
        .find_by_id(order_id)
        .await
        .map_err(|e| {
            error!("❌ [update_order] DB error: {:?}", e);
            e
        })?
        .ok_or_else(|| {
            warn!("⚠️ [update_order] order not found: {}", order_id);
            AppError::NotFound("Pesanan tidak ditemukan".to_string())
        })?;

    debug!("📋 [update_order] current status={:?}", order.status);

    match (&req.status, &role) {
        (OrderStatus::Purchased | OrderStatus::Shipped, Role::Jastiper)
            if order.jastiper_id != requester_id =>
        {
            return Err(AppError::Forbidden(
                "Hanya jastiper pemilik produk".to_string(),
            ));
        }
        (OrderStatus::Completed, Role::Titipers) if order.titipers_id != requester_id => {
            return Err(AppError::Forbidden(
                "Hanya titipers pemilik order".to_string(),
            ));
        }
        _ => {}
    }

    if req.status == OrderStatus::Shipped {
        if req.tracking_number.is_none() {
            return Err(AppError::UnprocessableEntity(
                "tracking_number wajib diisi saat status SHIPPED".to_string(),
            ));
        }
        if req.courier.is_none() {
            return Err(AppError::UnprocessableEntity(
                "courier wajib diisi saat status SHIPPED".to_string(),
            ));
        }
    }

    let mut machine = OrderMachine::from_status(&order.status);
    let new_status = machine.update_status(role, &req.status)?;

    let result = order_repo
        .update(
            order_id,
            &new_status,
            UpdateOrderParams {
                changed_by: &requester_id.to_string(),
                actor_role: role,
                notes: req.notes.as_deref(),
                tracking_number: req.tracking_number.as_deref(),
                courier: req.courier.as_deref(),
                cancellation_reason: req.cancellation_reason.as_deref(),
            },
        )
        .await
        .map_err(|e| {
            error!("❌ [update_order] DB error: {:?}", e);
            e
        })?;

    info!(
        "✅ [update_status] order_id={} status updated to {:?}",
        order_id, req.status
    );
    Ok(result)
}

// ── cancel_status ─────────────────────────────────────────────────
pub async fn cancel_status(
    order_repo: &dyn OrderRepository,
    order_id: Uuid,
    requester_id: Uuid,
    role: &Role,
    req: UpdateStatusRequest, // biarkan param ini untuk notes, cancellation_reason dll
) -> Result<Order, AppError> {
    let order = order_repo
        .find_by_id(order_id)
        .await
        .map_err(|e| {
            error!("❌ [update_order] DB error: {:?}", e);
            e
        })?
        .ok_or_else(|| {
            warn!("⚠️ [update_order] order not found: {}", order_id);
            AppError::NotFound("Pesanan tidak ditemukan".to_string())
        })?;

    debug!("📋 [cancel_status] current status={:?}", order.status);

    let machine = OrderMachine::from_status(&order.status);
    let new_status = machine.cancel(role)?;

    debug!(
        "📋 [cancel_status] new status={:?}", // ← perbaiki debug
        new_status
    );

    let result = order_repo
        .update(
            order_id,
            &new_status, // ← tetap pakai new_status
            UpdateOrderParams {
                changed_by: &requester_id.to_string(),
                actor_role: role,
                notes: req.notes.as_deref(),
                tracking_number: req.tracking_number.as_deref(),
                courier: req.courier.as_deref(),
                cancellation_reason: req.cancellation_reason.as_deref(),
            },
        )
        .await
        .map_err(|e| {
            error!("❌ [update_order] DB error: {:?}", e);
            e
        })?;

    info!(
        "✅ [cancel_status] order_id={} status updated to {:?}", // ← ganti req.status jadi new_status
        order_id, new_status
    );
    Ok(result)
}

// ── payment ──────────────────────────────────────────────────────
pub async fn payment(
    order_repo: &dyn OrderRepository,
    wallet_client: &dyn WalletClient,
    titipers_id: Uuid,
    order_id: Uuid,
) -> Result<Order, AppError> {
    let order = order_repo
        .find_by_id(order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    if order.titipers_id != titipers_id {
        return Err(AppError::Forbidden("Bukan pemilik order".to_string()));
    }

    if order.status != OrderStatus::Pending {
        return Err(AppError::Conflict(format!(
            "Status harus PENDING, sekarang {:?}",
            order.status
        )));
    }

    debug!(
        "💵 [payment] deduct wallet balance for titipers_id={} amount={}",
        titipers_id, order.total_price
    );
    let desc = format!("Pembayaran Order #{}", order_id);
    if let Err(e) = wallet_client
        .deduct_wallet(titipers_id, order_id, order.total_price, &desc)
        .await
    {
        error!("❌ [payment] deduct_wallet gagal: {:?}", e);
        return Err(e);
    }
    info!("✅ [payment] wallet deducted, wallet service sudah menginfirmasi pembayaran");

    let result = update_status(
        order_repo,
        order_id,
        titipers_id,
        &Role::System,
        UpdateStatusRequest {
            status: OrderStatus::Paid,
            notes: Some("Pembayaran berhasil dilakukan titipers".to_string()),
            tracking_number: None,
            courier: None,
            cancellation_reason: None,
        },
    )
    .await
    .map_err(|e| {
        error!("❌ [payment] update_status gagal: {:?}", e);
        e
    })?;

    Ok(result)
}

// ── confirm_order ─────────────────────────────────────────────────
pub async fn confirm_order(
    order_repo: &dyn OrderRepository,
    titipers_id: Uuid,
    order_id: Uuid,
) -> Result<Order, AppError> {
    let result = update_status(
        order_repo,
        order_id,
        titipers_id,
        &Role::Titipers,
        UpdateStatusRequest {
            status: OrderStatus::Completed,
            notes: Some("Order sudah diterima oleh titipers".to_string()),
            tracking_number: None,
            courier: None,
            cancellation_reason: None,
        },
    )
    .await
    .map_err(|e| {
        error!("❌ [payment] update_status gagal: {:?}", e);
        e
    })?;

    Ok(result)
}

// ── purchased ─────────────────────────────────────────────────────
pub async fn purchased(
    order_repo: &dyn OrderRepository,
    order_id: Uuid,
    jastiper_id: Uuid,
) -> Result<Order, AppError> {
    let result = update_status(
        order_repo,
        order_id,
        jastiper_id,
        &Role::Jastiper,
        UpdateStatusRequest {
            status: OrderStatus::Purchased,
            notes: Some("Order sudah dibeli oleh jastiper".to_string()),
            tracking_number: None,
            courier: None,
            cancellation_reason: None,
        },
    )
    .await
    .map_err(|e| {
        error!("❌ [payment] update_status gagal: {:?}", e);
        e
    })?;

    Ok(result)
}

// ── shipped ─────────────────────────────────────────────────────
pub async fn shipped(
    order_repo: &dyn OrderRepository,
    order_id: Uuid,
    jastiper_id: Uuid,
    req: ShippedRequest,
) -> Result<Order, AppError> {
    let result = update_status(
        order_repo,
        order_id,
        jastiper_id,
        &Role::Jastiper,
        UpdateStatusRequest {
            status: OrderStatus::Shipped,
            notes: Some("Order sudah dikirim oleh jastiper".to_string()),
            tracking_number: req.tracking_number,
            courier: req.courier,
            cancellation_reason: None,
        },
    )
    .await
    .map_err(|e| {
        error!("❌ [payment] update_status gagal: {:?}", e);
        e
    })?;

    Ok(result)
}

// ── get_order_history ─────────────────────────────────────────────
pub async fn get_order_history(
    order_repo: &dyn OrderRepository,
    order_status_repo: &dyn OrderStatusHistoryRepository,
    order_id: Uuid,
    requester_id: Uuid,
) -> Result<Vec<OrderStatusHistory>, AppError> {
    debug!(
        "📜 [get_order_history] order_id={} requester_id={}",
        order_id, requester_id
    );

    get_order(order_repo, order_id, requester_id).await?;

    let history = order_status_repo
        .get_status_history(order_id)
        .await
        .map_err(|e| {
            error!("❌ [get_order_history] DB error: {:?}", e);
            e
        })?;

    debug!("✅ [get_order_history] found {} entries", history.len());
    Ok(history)
}

// ── cancel_order ──────────────────────────────────────────────────
pub async fn cancel_order(
    order_repo: &dyn OrderRepository,
    inventory_client: &dyn InventoryClient,
    wallet_client: &dyn WalletClient,
    order_id: Uuid,
    requester_id: Uuid,
    role: &Role,
    req: CancelRequest,
) -> Result<Order, AppError> {
    info!(
        "🚫 [cancel_order] order_id={} requester_id={} role={}",
        order_id, requester_id, role
    );

    let updated = cancel_status(
        order_repo,
        order_id,
        requester_id,
        role,
        UpdateStatusRequest {
            status: OrderStatus::Refunding,
            notes: Some(format!("Order dibatalkan oleh {}", role).to_string()),
            tracking_number: None,
            courier: None,
            cancellation_reason: Some(req.cancellation_reason.clone()),
        },
    )
    .await
    .map_err(|e| {
        error!("❌ [payment] update_status gagal: {:?}", e);
        e
    })?;
    info!(
        "✅ [cancel_order] order status updated to REFUNDING, proceeding with stock release and wallet refund"
    );

    let pid: Uuid = serde_json::from_value(updated.product_snapshot["product_id"].clone())
        .unwrap_or(updated.product_id);
    debug!(
        "📦 [cancel_order] releasing stock product_id={} qty={}",
        pid, updated.quantity
    );
    let _ = inventory_client
        .release_stock(pid, order_id, updated.quantity)
        .await;

    let rd = format!("Refund Order #{} - dibatalkan", order_id);
    debug!(
        "💳 [cancel_order] refunding wallet titipers_id={} amount={}",
        updated.titipers_id, updated.total_price
    );
    let _ = wallet_client
        .refund_wallet(updated.titipers_id, order_id, updated.total_price, &rd)
        .await;

    Ok(updated)
}

// ── my_purchases & my_sales ───────────────────────────────────────
pub async fn my_purchases(
    order_repo: &dyn OrderRepository,
    titipers_id: Uuid,
    params: PaginationParams,
) -> Result<(Vec<Order>, i64), AppError> {
    debug!(
        "📋 [my_purchases] titipers_id={} page={:?} limit={:?}",
        titipers_id, params.page, params.limit
    );

    let order_filter = OrderFilter {
        titipers_id: Some(titipers_id),
        ..Default::default()
    };
    let filter = Some(&order_filter);

    let result = order_repo.find_all(filter, &params).await.map_err(|e| {
        error!("❌ [my_purchases] DB error: {:?}", e);
        e
    })?;

    debug!("✅ [my_purchases] found {} orders", result.0.len());
    Ok(result)
}

pub async fn my_sales(
    order_repo: &dyn OrderRepository,
    jastiper_id: Uuid,
    params: PaginationParams,
) -> Result<(Vec<Order>, i64), AppError> {
    debug!(
        "📋 [my_sales] jastiper_id={} page={:?} limit={:?}",
        jastiper_id, params.page, params.limit
    );

    let order_filter = OrderFilter {
        jastiper_id: Some(jastiper_id),
        ..Default::default()
    };
    let filter = Some(&order_filter);

    let result = order_repo.find_all(filter, &params).await.map_err(|e| {
        error!("❌ [my_sales] DB error: {:?}", e);
        e
    })?;

    debug!("✅ [my_sales] found {} orders", result.0.len());
    Ok(result)
}
