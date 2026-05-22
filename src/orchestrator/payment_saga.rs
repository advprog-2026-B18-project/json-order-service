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
use crate::services::wallet_client::WalletClient;

// PAYMENT SAGA

// Flow:
//   Step 1: DeductWallet       → wallet_client.deduct_wallet()
//   Step 2: UpdateStatusToPaid → order_repo.update(status=PAID)

pub struct PaymentContext {
    pub titipers_id: Uuid,
    pub order_id: Uuid,
    pub total_price: i64,

    // fill while saga running
    pub wallet_transaction_id: Option<String>,
    pub updated_order: Option<Order>,
}

pub struct UpdateStatusToPaidStep {
    pub order_repo: Arc<dyn OrderRepository + Send + Sync>,
}

#[async_trait]
impl SagaStep for UpdateStatusToPaidStep {
    type Context = PaymentContext;

    async fn execute(&self, ctx: &mut PaymentContext) -> Result<(), AppError> {
        let order = self
            .order_repo
            .update(
                ctx.order_id,
                &OrderStatus::Paid,
                UpdateOrderParams {
                    changed_by: &ctx.titipers_id.to_string(),
                    actor_role: &Role::System,
                    notes: Some("Pembayaran berhasil dilakukan titipers"),
                    tracking_number: None,
                    courier: None,
                    cancellation_reason: None,
                },
            )
            .await
            .map_err(|e| {
                error!(
                    "❌ [UpdateStatusToPaidStep] update status ke PAID gagal order_id={}: {:?}",
                    ctx.order_id, e
                );
                e
            })?;

        info!(
            "✅ [UpdateStatusToPaidStep] status order_id={} berhasil diupdate ke PAID",
            ctx.order_id
        );
        ctx.updated_order = Some(order);
        Ok(())
    }

    async fn compensate(&self, ctx: &mut PaymentContext) -> Result<(), AppError> {
        use crate::models::order::UpdateOrderParams;
        use crate::models::role::Role;

        error!(
            "↩️  [UpdateStatusToPaidStep] revert status ke PENDING order_id={}",
            ctx.order_id
        );

        self.order_repo
            .update(
                ctx.order_id,
                &OrderStatus::Pending,
                UpdateOrderParams {
                    changed_by: "system",
                    actor_role: &Role::System,
                    notes: Some("Revert PAID → PENDING karena deduct wallet gagal"),
                    tracking_number: None,
                    courier: None,
                    cancellation_reason: None,
                },
            )
            .await
            .map_err(|e| {
                error!(
                    "🚨 [UpdateStatusToPaidStep] revert ke PENDING gagal order_id={}: {:?}",
                    ctx.order_id, e
                );
                e
            })?;

        Ok(())
    }

    fn name(&self) -> &'static str {
        "update_status_to_paid"
    }
}

pub struct DeductWalletStep {
    pub wallet_client: Arc<dyn WalletClient + Send + Sync>,
}

#[async_trait]
impl SagaStep for DeductWalletStep {
    type Context = PaymentContext;

    async fn execute(&self, ctx: &mut PaymentContext) -> Result<(), AppError> {
        let desc = format!("Pembayaran Order #{}", ctx.order_id);

        let result = self
            .wallet_client
            .deduct_wallet(ctx.titipers_id, ctx.order_id, ctx.total_price, &desc)
            .await
            .map_err(|e| {
                error!(
                    "❌ [DeductWalletStep] deduct_wallet gagal titipers_id={} amount={}: {:?}",
                    ctx.titipers_id, ctx.total_price, e
                );
                e
            })?;

        ctx.wallet_transaction_id = Some(result.transaction_id);
        info!(
            "✅ [DeductWalletStep] wallet berhasil dipotong titipers_id={} amount={} txn_id={:?}",
            ctx.titipers_id, ctx.total_price, ctx.wallet_transaction_id
        );
        Ok(())
    }

    async fn compensate(&self, ctx: &mut PaymentContext) -> Result<(), AppError> {
        info!(
            "↩️  [DeductWalletStep] no-op — deduct belum terjadi order_id={}",
            ctx.order_id
        );
        Ok(())
    }

    fn name(&self) -> &'static str {
        "deduct_wallet"
    }
}
