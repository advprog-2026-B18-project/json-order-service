use std::sync::Arc;

use async_trait::async_trait;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::order::{Order, UpdateOrderParams};
use crate::models::order_status_history::OrderStatus;
use crate::models::role::Role;
use crate::orchestrator::SagaStep;
use crate::repositories::order_repository::OrderRepository;
use crate::services::inventory_client::InventoryClient;
use crate::services::wallet_client::WalletClient;

// CANCEL SAGA

// Flow:
//   Step 1: UpdateStatusToRefunding → order_repo.update(status=REFUNDING)
//   Step 2: ReleaseStock            → inventory_client.release_stock()
//   Step 3: RefundWallet            → wallet_client.refund_wallet() [async]

pub struct CancelOrderContext {
    // Input
    pub order_id: Uuid,
    pub requester_id: Uuid,
    pub role: Role,
    pub product_id: Uuid,
    pub titipers_id: Uuid,
    pub status: OrderStatus,
    pub quantity: i32,
    pub total_price: i64,
    pub cancellation_reason: String,

    // fill while saga running
    pub status_set_to_refunding: bool,
    pub stock_released: bool,
    pub refunding_order: Option<Order>,
}

pub struct UpdateStatusToRefundingStep {
    pub order_repo: Arc<dyn OrderRepository + Send + Sync>,
}

#[async_trait]
impl SagaStep for UpdateStatusToRefundingStep {
    type Context = CancelOrderContext;

    async fn execute(&self, ctx: &mut CancelOrderContext) -> Result<(), AppError> {
        use crate::models::order_state::OrderMachine;

        let order = self
            .order_repo
            .find_by_id(ctx.order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

        let machine = OrderMachine::from_status(&order.status);
        let new_status = machine.cancel(&ctx.role)?;

        let updated = self
            .order_repo
            .update(
                ctx.order_id,
                &new_status,
                UpdateOrderParams {
                    changed_by: &ctx.requester_id.to_string(),
                    actor_role: &ctx.role,
                    notes: Some(&format!("Order dibatalkan oleh {}", ctx.role)),
                    tracking_number: None,
                    courier: None,
                    cancellation_reason: Some(&ctx.cancellation_reason),
                },
            )
            .await
            .map_err(|e| {
                error!(
                    "❌ [UpdateStatusToRefundingStep] update gagal order_id={}: {:?}",
                    ctx.order_id, e
                );
                e
            })?;

        ctx.status_set_to_refunding = true;
        ctx.refunding_order = Some(updated);
        info!(
            "✅ [UpdateStatusToRefundingStep] status order_id={} diset ke REFUNDING",
            ctx.order_id
        );
        Ok(())
    }

    async fn compensate(&self, ctx: &mut CancelOrderContext) -> Result<(), AppError> {
        if !ctx.status_set_to_refunding {
            return Ok(());
        }

        warn!(
            "↩️  [UpdateStatusToRefundingStep] revert ke PENDING order_id={}",
            ctx.order_id
        );

        self.order_repo
            .update(
                ctx.order_id,
                &OrderStatus::Pending,
                UpdateOrderParams {
                    changed_by: "system",
                    actor_role: &Role::System,
                    notes: Some("Revert REFUNDING → PENDING karena proses cancel gagal"),
                    tracking_number: None,
                    courier: None,
                    cancellation_reason: None,
                },
            )
            .await
            .map_err(|e| {
                error!(
                    "🚨 [UpdateStatusToRefundingStep] revert ke PENDING GAGAL order_id={}: {:?}",
                    ctx.order_id, e
                );
                e
            })?;

        ctx.status_set_to_refunding = false;
        ctx.refunding_order = None;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "update_status_to_refunding"
    }
}

pub struct ReleaseStockStep {
    pub inventory_client: Arc<dyn InventoryClient + Send + Sync>,
}

#[async_trait]
impl SagaStep for ReleaseStockStep {
    type Context = CancelOrderContext;

    async fn execute(&self, ctx: &mut CancelOrderContext) -> Result<(), AppError> {
        self.inventory_client
            .release_stock(ctx.product_id, ctx.order_id, ctx.quantity)
            .await
            .map_err(|e| {
                error!(
                    "❌ [ReleaseStockStep] release_stock gagal product_id={} order_id={}: {:?}",
                    ctx.product_id, ctx.order_id, e
                );
                e
            })?;

        ctx.stock_released = true;
        info!(
            "✅ [ReleaseStockStep] stok dilepas product_id={} qty={}",
            ctx.product_id, ctx.quantity
        );
        Ok(())
    }

    async fn compensate(&self, ctx: &mut CancelOrderContext) -> Result<(), AppError> {
        if !ctx.stock_released {
            return Ok(());
        }

        warn!(
            "↩️  [ReleaseStockStep] re-reserve stok product_id={} qty={}",
            ctx.product_id, ctx.quantity
        );

        self.inventory_client
            .reserve_stock(ctx.product_id, ctx.order_id, ctx.quantity)
            .await
            .map_err(|e| {
                error!(
                    "🚨 [ReleaseStockStep] re-reserve GAGAL — stok product_id={} sudah lepas tanpa refund! {:?}",
                    ctx.product_id, e
                );
                e
            })?;

        ctx.stock_released = false;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "release_stock"
    }
}

pub struct RefundWalletStep {
    pub wallet_client: Arc<dyn WalletClient + Send + Sync>,
}

#[async_trait]
impl SagaStep for RefundWalletStep {
    type Context = CancelOrderContext;

    async fn execute(&self, ctx: &mut CancelOrderContext) -> Result<(), AppError> {
        if ctx.status == OrderStatus::Pending {
            info!(
                "↩️  [RefundWalletStep] skip — order masih PENDING, wallet belum di-deduct order_id={}",
                ctx.order_id
            );
            return Ok(());
        }

        let desc = format!("Refund Order #{} — dibatalkan", ctx.order_id);

        self.wallet_client
            .refund_wallet(ctx.titipers_id, ctx.order_id, ctx.total_price, &desc)
            .await
            .map_err(|e| {
                error!(
                    "❌ [RefundWalletStep] refund_wallet gagal titipers_id={} amount={}: {:?}",
                    ctx.titipers_id, ctx.total_price, e
                );
                e
            })?;

        info!(
            "✅ [RefundWalletStep] refund request dikirim titipers_id={} amount={}",
            ctx.titipers_id, ctx.total_price
        );
        Ok(())
    }

    async fn compensate(&self, ctx: &mut CancelOrderContext) -> Result<(), AppError> {
        info!(
            "↩️  [RefundWalletStep] no-op — refund request gagal dikirim, tidak ada state yang berubah order_id={}",
            ctx.order_id
        );
        Ok(())
    }

    fn name(&self) -> &'static str {
        "refund_wallet"
    }
}
