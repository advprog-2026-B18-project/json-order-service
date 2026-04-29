use crate::error::AppError;
use crate::models::filter_pagination::{OrderFilter, PaginationParams};
use crate::models::order::{CancelRequest, Order, OrderStatus, UpdateStatusRequest};
use crate::models::role::Role;
use crate::ports::inventory_client::InventoryClient;
use crate::ports::order_repository::OrderRepository;
use crate::ports::wallet_client::WalletClient;
use crate::services::order::cancel_status;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub async fn get_all(
    order_repo: &dyn OrderRepository,
    order_filter: &OrderFilter,
    params: &PaginationParams,
    role: &Role,
) -> Result<(Vec<Order>, i64), AppError> {
    debug!(
        "📋 [all_purchases] role={} page={:?} limit={:?}",
        role, params.page, params.limit
    );

    if *role != Role::Admin {
        warn!(
            "⚠️ [admin_get_order] forbidden: requester_role={} bukan titipers/jastiper",
            role
        );
        return Err(AppError::Forbidden(
            "Anda tidak memiliki akses ke pesanan ini".to_string(),
        ));
    }

    let filter = Some(order_filter);
    let result = order_repo.find_all(filter, &params).await.map_err(|e| {
        error!("❌ [all_purchases] DB error: {:?}", e);
        e
    })?;

    debug!("✅ [all_purchases] found {} orders", result.0.len());
    Ok(result)
}

pub async fn get_order(
    order_repo: &dyn OrderRepository,
    order_id: Uuid,
    role: &Role,
) -> Result<Order, AppError> {
    debug!("🔍 [admin_get_order] order_id={} role={}", order_id, role);

    if *role != Role::Admin {
        warn!(
            "⚠️ [admin_get_order] forbidden: requester_role={} bukan titipers/jastiper",
            role
        );
        return Err(AppError::Forbidden(
            "Anda tidak memiliki akses ke pesanan ini".to_string(),
        ));
    }

    let order = order_repo
        .find_by_id(order_id)
        .await
        .map_err(|e| {
            error!("❌ [admin_get_order] DB error: {:?}", e);
            e
        })?
        .ok_or_else(|| {
            warn!("⚠️ [admin_get_order] order not found: {}", order_id);
            AppError::NotFound("Pesanan tidak ditemukan".to_string())
        })?;

    debug!(
        "✅ [admin_get_order] found order_id={} status={:?}",
        order.order_id, order.status
    );
    Ok(order)
}

pub async fn force_cancel(
    order_repo: &dyn OrderRepository,
    inventory_client: &dyn InventoryClient,
    wallet_client: &dyn WalletClient,
    order_id: Uuid,
    requester_id: Uuid,
    role: &Role,
    req: CancelRequest,
) -> Result<Order, AppError> {
    info!(
        "🚫 [force_cancel_order] order_id={} requester_id={} role={}",
        order_id, requester_id, role
    );

    if *role != Role::Admin {
        warn!(
            "⚠️ [force_cancel_order] forbidden: requester_role={} bukan titipers/jastiper/admin",
            role
        );
        return Err(AppError::Forbidden(
            "Anda tidak memiliki akses ke pesanan ini".to_string(),
        ));
    }

    let updated = cancel_status(
        order_repo,
        order_id,
        requester_id,
        &Role::Admin,
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
        error!("❌ [force_cancel_order] update_status gagal: {:?}", e);
        e
    })?;
    info!(
        "✅ [force_cancel_order] order status updated to REFUNDING, proceeding with stock release and wallet refund"
    );

    let pid: Uuid = serde_json::from_value(updated.product_snapshot["product_id"].clone())
        .unwrap_or(updated.product_id);
    debug!(
        "📦 [force_cancel_order] releasing stock product_id={} qty={}",
        pid, updated.quantity
    );
    let _ = inventory_client
        .release_stock(pid, order_id, updated.quantity)
        .await;

    let rd = format!("Refund Order #{} - dibatalkan", order_id);
    debug!(
        "💳 [force_cancel_order] refunding wallet titipers_id={} amount={}",
        updated.titipers_id, updated.total_price
    );
    let _ = wallet_client
        .refund_wallet(updated.titipers_id, order_id, updated.total_price, &rd)
        .await;

    Ok(updated)
}
