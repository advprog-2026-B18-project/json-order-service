use crate::error::AppError;
use crate::infrastructure::publisher::CheckoutPublisher;
use crate::models::checkout_request::CheckoutRequest;
use crate::models::filter_pagination::{OrderFilter, OrderQueryParams};
use crate::models::order::{
    CancelRequest, CreateOrderRequest, Order, PriceBreakdown, ShippedRequest, UpdateOrderParams,
    UpdateStatusRequest,
};
use crate::models::order_state::OrderMachine;
use crate::models::order_status_history::{OrderStatus, OrderStatusHistory};
use crate::models::role::Role;
use crate::orchestrator::SagaOrchestrator;
use crate::orchestrator::cancel_order_saga::{
    CancelOrderContext, RefundWalletStep, ReleaseStockStep, UpdateStatusToRefundingStep,
};
use crate::orchestrator::confirm_order_saga::{
    ConfirmOrderContext, SendConfirmationProductStep, TransferEarningsStep,
    UpdateStatusToCompletedStep,
};
use crate::orchestrator::payment_saga::{DeductWalletStep, PaymentContext, UpdateStatusToPaidStep};
use crate::repositories::order_repository::OrderRepository;
use crate::repositories::order_status_history_repository::OrderStatusHistoryRepository;
use crate::services::inventory_client::InventoryClient;
use crate::services::wallet_client::WalletClient;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// ── checkout ──────────────────────────────────────────────────────
pub async fn checkout(
    order_repo: Arc<dyn OrderRepository + Send + Sync>,
    inventory_client: Arc<dyn InventoryClient + Send + Sync>,
    checkout_publisher: Arc<dyn CheckoutPublisher + Send + Sync>,
    titipers_id: Uuid,
    req: CreateOrderRequest,
) -> Result<Order, AppError> {
    info!(
        "🛒 [checkout] titipers_id={} product_id={} qty={}",
        titipers_id, req.product_id, req.quantity
    );

    // 1. Fetch product
    let product = inventory_client
        .fetch_product(req.product_id)
        .await
        .map_err(|e| {
            error!("❌ [checkout] fetch_product gagal: {:?}", e);
            e
        })?;

    let jastiper_id: Uuid = serde_json::from_value(product["jastiper"]["user_id"].clone())
        .map_err(|_| AppError::Internal)?;

    if titipers_id == jastiper_id {
        return Err(AppError::Forbidden(
            "Jastiper tidak dapat membeli produk milik sendiri".to_string(),
        ));
    }

    // 2. Hitung harga
    let unit_price = product["price"].as_i64().unwrap_or(0);
    let service_fee = product["service_fee"].as_i64().unwrap_or(0);
    let total_price = (unit_price + service_fee) * req.quantity as i64;

    // 3. Buat snapshot
    let snapshot = serde_json::json!({
        "product_id":     req.product_id,
        "name":           product["name"],
        "description":    product["description"],
        "image_url":      product["images"][0],
        "origin_country": product["originCountry"],
        "purchase_date":  product["purchaseDate"],
        "unit_price":     unit_price,
        "service_fee":    service_fee,
    });

    // 4. Buat order Reserving di DB
    let order = order_repo
        .create(
            titipers_id,
            jastiper_id,
            req.clone(),
            snapshot,
            PriceBreakdown {
                unit_price,
                service_fee,
                total_price,
            },
        )
        .await
        .map_err(|e| {
            error!("❌ [checkout] create order gagal: {:?}", e);
            e
        })?;

    // 5. Publish ke queue
    let checkout_request = CheckoutRequest {
        order_id: order.order_id,
        titipers_id,
        jastiper_id,
        req,
        product,
        idempotency_key: order.order_id,
    };

    checkout_publisher
        .publish(&checkout_request)
        .await
        .map_err(|e| {
            error!("❌ [checkout] publish ke queue gagal: {e:?}");
            e
        })?;

    info!("✅ [checkout] order queued order_id={}", order.order_id);
    Ok(order)
}

// ── get_order ─────────────────────────────────────────────────────
pub async fn get_order(
    order_repo: Arc<dyn OrderRepository + Send + Sync>,
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
    order_repo: Arc<dyn OrderRepository + Send + Sync>,
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
    order_repo: Arc<dyn OrderRepository + Send + Sync>,
    order_id: Uuid,
    requester_id: Uuid,
    role: &Role,
    req: UpdateStatusRequest,
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

    debug!("📋 [cancel_status] new status={:?}", new_status);

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
        "✅ [cancel_status] order_id={} status updated to {:?}",
        order_id, new_status
    );
    Ok(result)
}

// ── payment ──────────────────────────────────────────────────────
pub async fn payment(
    order_repo: Arc<dyn OrderRepository + Send + Sync>,
    wallet_client: Arc<dyn WalletClient + Send + Sync>,
    titipers_id: Uuid,
    order_id: Uuid,
) -> Result<Order, AppError> {
    info!(
        "💳 [payment] titipers_id={} order_id={}",
        titipers_id, order_id
    );

    // Validation
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

    // Saga step
    let mut ctx = PaymentContext {
        titipers_id,
        order_id,
        total_price: order.total_price,
        wallet_transaction_id: None,
        updated_order: None,
    };

    let saga = SagaOrchestrator::new("payment")
        .step(DeductWalletStep {
            wallet_client: Arc::clone(&wallet_client),
        })
        .step(UpdateStatusToPaidStep {
            order_repo: Arc::clone(&order_repo),
        });

    saga.run(&mut ctx).await?;

    let result = ctx
        .updated_order
        .expect("UpdateStatusToPaidStep harus mengisi updated_order");
    info!("✅ [payment] selesai order_id={}", order_id);
    Ok(result)
}

// ── confirm_order ──────────────────────f───────────────────────────
pub async fn confirm_order(
    order_repo: Arc<dyn OrderRepository + Send + Sync>,
    wallet_client: Arc<dyn WalletClient + Send + Sync>,
    inventory_client: Arc<dyn InventoryClient + Send + Sync>,
    titipers_id: Uuid,
    order_id: Uuid,
) -> Result<Order, AppError> {
    info!(
        "✅ [confirm_order] titipers_id={} order_id={}",
        titipers_id, order_id
    );

    // Validasi
    let order = order_repo
        .find_by_id(order_id)
        .await
        .map_err(|e| {
            error!("❌ [confirm_order] DB error: {:?}", e);
            e
        })?
        .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;
    if order.titipers_id != titipers_id {
        return Err(AppError::Forbidden(
            "Hanya titipers pemilik order yang dapat mengkonfirmasi".to_string(),
        ));
    }
    if order.status != OrderStatus::Shipped {
        return Err(AppError::Conflict(format!(
            "Status harus SHIPPED untuk dikonfirmasi, sekarang {:?}",
            order.status
        )));
    }

    // Saga step
    let mut ctx = ConfirmOrderContext {
        titipers_id,
        jastiper_id: order.jastiper_id,
        order_id,
        product_id: order.product_id,
        total_price: order.total_price,
        earnings_transaction_id: None,
        updated_order: None,
    };

    let saga = SagaOrchestrator::new("confirm_order")
        .step(UpdateStatusToCompletedStep {
            order_repo: Arc::clone(&order_repo),
        })
        .step(TransferEarningsStep {
            wallet_client: Arc::clone(&wallet_client),
        })
        .step(SendConfirmationProductStep {
            inventory_client: Arc::clone(&inventory_client),
        });

    saga.run(&mut ctx).await?;

    let result = ctx
        .updated_order
        .expect("UpdateStatusToCompletedStep harus mengisi updated_order");
    info!("✅ [confirm_order] selesai order_id={}", order_id);
    Ok(result)
}

// ── purchased ─────────────────────────────────────────────────────
pub async fn purchased(
    order_repo: Arc<dyn OrderRepository + Send + Sync>,
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
    order_repo: Arc<dyn OrderRepository + Send + Sync>,
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
    order_repo: Arc<dyn OrderRepository + Send + Sync>,
    order_status_repo: Arc<dyn OrderStatusHistoryRepository + Send + Sync>,
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
    order_repo: Arc<dyn OrderRepository + Send + Sync>,
    inventory_client: Arc<dyn InventoryClient + Send + Sync>,
    wallet_client: Arc<dyn WalletClient + Send + Sync>,
    order_id: Uuid,
    requester_id: Uuid,
    role: &Role,
    req: CancelRequest,
) -> Result<Order, AppError> {
    info!(
        "🚫 [cancel_order] order_id={} requester_id={} role={}",
        order_id, requester_id, role
    );

    // Validasi
    let order = order_repo
        .find_by_id(order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    match role {
        Role::Titipers if order.titipers_id != requester_id => {
            return Err(AppError::Forbidden("Bukan pemilik order".to_string()));
        }
        Role::Jastiper if order.jastiper_id != requester_id => {
            return Err(AppError::Forbidden(
                "Bukan jastiper pemilik produk".to_string(),
            ));
        }
        _ => {}
    }

    let product_id: Uuid = serde_json::from_value(order.product_snapshot["product_id"].clone())
        .unwrap_or(order.product_id);

    // Saga step
    let mut ctx = CancelOrderContext {
        order_id,
        requester_id,
        role: role.clone(),
        product_id,
        titipers_id: order.titipers_id,
        status: order.status,
        quantity: order.quantity,
        total_price: order.total_price,
        cancellation_reason: req.cancellation_reason,
        status_set_to_refunding: false,
        stock_released: false,
        refunding_order: None,
    };

    let saga = SagaOrchestrator::new("cancel_order")
        .step(UpdateStatusToRefundingStep {
            order_repo: Arc::clone(&order_repo),
        })
        .step(ReleaseStockStep {
            inventory_client: Arc::clone(&inventory_client),
        })
        .step(RefundWalletStep {
            wallet_client: Arc::clone(&wallet_client),
        });

    saga.run(&mut ctx).await?;

    let result = ctx
        .refunding_order
        .expect("UpdateStatusToCancelledStep harus mengisi refunding_order");
    info!("✅ [cancel_order] selesai order_id={}", order_id);
    Ok(result)
}

// ── my_purchases & my_sales ───────────────────────────────────────
pub async fn my_purchases(
    order_repo: Arc<dyn OrderRepository + Send + Sync>,
    titipers_id: Uuid,
    params: OrderQueryParams,
) -> Result<(Vec<Order>, i64), AppError> {
    debug!(
        "📋 [my_purchases] titipers_id={} page={:?} limit={:?}",
        titipers_id, params.pagination.page, params.pagination.limit
    );

    let order_filter = OrderFilter {
        titipers_id: Some(titipers_id),
        status: params.filter.status,
        date_from: params.filter.date_from,
        date_to: params.filter.date_to,
        ..Default::default()
    };
    let filter = Some(&order_filter);

    let result = order_repo
        .find_all(filter, &params.pagination)
        .await
        .map_err(|e| {
            error!("❌ [my_purchases] DB error: {:?}", e);
            e
        })?;

    debug!("✅ [my_purchases] found {} orders", result.0.len());
    Ok(result)
}

pub async fn my_sales(
    order_repo: Arc<dyn OrderRepository + Send + Sync>,
    jastiper_id: Uuid,
    params: OrderQueryParams,
) -> Result<(Vec<Order>, i64), AppError> {
    debug!(
        "📋 [my_sales] jastiper_id={} page={:?} limit={:?}",
        jastiper_id, params.pagination.page, params.pagination.limit
    );

    let order_filter = OrderFilter {
        jastiper_id: Some(jastiper_id),
        status: params.filter.status,
        date_from: params.filter.date_from,
        date_to: params.filter.date_to,
        ..Default::default()
    };
    let filter = Some(&order_filter);

    let result = order_repo
        .find_all(filter, &params.pagination)
        .await
        .map_err(|e| {
            error!("❌ [my_sales] DB error: {:?}", e);
            e
        })?;

    debug!("✅ [my_sales] found {} orders", result.0.len());
    Ok(result)
}
