use std::sync::Arc;

use async_trait::async_trait;
use tracing::{error, info};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::order::Order;
use crate::models::order::UpdateOrderParams;
use crate::models::order_status_history::OrderStatus;
use crate::models::role::Role;
use crate::orchestrator::SagaStep;
use crate::repositories::order_repository::OrderRepository;
use crate::services::inventory_client::InventoryClient;
use crate::services::wallet_client::WalletClient;

// CONFIRM SAGA

// Flow:
//   Step 1: UpdateStatusToCompleted
//   Step 2: TransferEarnings
//   Step 3: SendConfirmationProduct

pub struct ConfirmOrderContext {
    pub titipers_id: Uuid,
    pub jastiper_id: Uuid,
    pub order_id: Uuid,
    pub product_id: Uuid,
    pub total_price: i64,

    // fill while saga running
    pub earnings_transaction_id: Option<String>,
    pub updated_order: Option<Order>,
}

pub struct UpdateStatusToCompletedStep {
    pub order_repo: Arc<dyn OrderRepository + Send + Sync>,
}

#[async_trait]
impl SagaStep for UpdateStatusToCompletedStep {
    type Context = ConfirmOrderContext;

    async fn execute(&self, ctx: &mut ConfirmOrderContext) -> Result<(), AppError> {
        use crate::models::order::UpdateOrderParams;
        use crate::models::role::Role;

        let order = self
            .order_repo
            .update(
                ctx.order_id,
                &OrderStatus::Completed,
                UpdateOrderParams {
                    changed_by: &ctx.titipers_id.to_string(),
                    actor_role: &Role::Titipers,
                    notes: Some("Order sudah diterima oleh titipers"),
                    tracking_number: None,
                    courier: None,
                    cancellation_reason: None,
                },
            )
            .await
            .map_err(|e| {
                error!(
                    "❌ [UpdateStatusToCompletedStep] update status gagal order_id={}: {:?}",
                    ctx.order_id, e
                );
                e
            })?;

        info!(
            "✅ [UpdateStatusToCompletedStep] status order_id={} berhasil diupdate ke COMPLETED",
            ctx.order_id
        );
        ctx.updated_order = Some(order);
        Ok(())
    }

    async fn compensate(&self, ctx: &mut ConfirmOrderContext) -> Result<(), AppError> {
        error!(
            "↩️  [UpdateStatusToCompletedStep] revert status ke SHIPPED order_id={}",
            ctx.order_id
        );

        self.order_repo
            .update(
                ctx.order_id,
                &OrderStatus::Shipped,
                UpdateOrderParams {
                    changed_by: "system",
                    actor_role: &Role::System,
                    notes: Some("Revert COMPLETED → SHIPPED karena transfer earnings gagal"),
                    tracking_number: None,
                    courier: None,
                    cancellation_reason: None,
                },
            )
            .await
            .map_err(|e| {
                error!(
                    "🚨 [UpdateStatusToCompletedStep] revert gagal order_id={}: {:?}",
                    ctx.order_id, e
                );
                e
            })?;

        Ok(())
    }

    fn name(&self) -> &'static str {
        "update_status_to_completed"
    }
}

pub struct TransferEarningsStep {
    pub wallet_client: Arc<dyn WalletClient + Send + Sync>,
}

#[async_trait]
impl SagaStep for TransferEarningsStep {
    type Context = ConfirmOrderContext;

    async fn execute(&self, ctx: &mut ConfirmOrderContext) -> Result<(), AppError> {
        let desc = format!("Pendapatan Order #{}", ctx.order_id);

        let result = self
            .wallet_client
            .earnings_wallet(ctx.jastiper_id, ctx.order_id, &desc)
            .await
            .map_err(|e| {
                error!(
                    "❌ [TransferEarningsStep] earnings_wallet gagal jastiper_id={}: {:?}",
                    ctx.jastiper_id, e
                );
                e
            })?;

        ctx.earnings_transaction_id = Some(result.transaction_id);
        info!(
            "✅ [TransferEarningsStep] earnings ditransfer ke jastiper_id={} txn_id={:?}",
            ctx.jastiper_id, ctx.earnings_transaction_id
        );
        Ok(())
    }

    async fn compensate(&self, ctx: &mut ConfirmOrderContext) -> Result<(), AppError> {
        if let Some(ref txn_id) = ctx.earnings_transaction_id {
            error!(
                "↩️  [TransferEarningsStep] revert earnings jastiper_id={} txn_id={}",
                ctx.jastiper_id, txn_id
            );

            self.wallet_client
            .reverse_earnings(ctx.jastiper_id, ctx.order_id, txn_id, "Revert earnings order ")
            .await
            .map_err(|e| {
                error!(
                "🚨 [TransferEarningsStep] revert earnings GAGAL jastiper_id={} txn_id={}: {:?}",
                ctx.jastiper_id, txn_id, e
            );
                e
            })?;

            info!(
                "✅ [TransferEarningsStep] earnings berhasil direvert jastiper_id={} txn_id={}",
                ctx.jastiper_id, txn_id
            );
        } else {
            info!(
                "↩️  [TransferEarningsStep] no-op — transfer belum terjadi order_id={}",
                ctx.order_id
            );
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "transfer_earnings_to_jastiper"
    }
}

pub struct SendConfirmationProductStep {
    pub inventory_client: Arc<dyn InventoryClient + Send + Sync>,
}

#[async_trait]
impl SagaStep for SendConfirmationProductStep {
    type Context = ConfirmOrderContext;

    async fn execute(&self, ctx: &mut ConfirmOrderContext) -> Result<(), AppError> {
        self.inventory_client
            .confirm_order_received(ctx.product_id, ctx.order_id)
            .await
            .map_err(|e| {
                error!(
                    "❌ [SendConfirmationProductStep] reserve_stock gagal: {:?}",
                    e
                );
                e
            })?;

        info!(
            "✅ [SendConfirmationProductStep] konfirmasi terkirim ke inventory product_id={} order_id={}",
            ctx.product_id, ctx.order_id
        );
        Ok(())
    }

    async fn compensate(&self, ctx: &mut ConfirmOrderContext) -> Result<(), AppError> {
        info!(
            "↩️  [SendConfirmationProductStep] no-op — konfirmasi inventory tidak perlu direvert order_id={}",
            ctx.order_id
        );
        Ok(())
    }

    fn name(&self) -> &'static str {
        "send_confirmation_product"
    }
}
