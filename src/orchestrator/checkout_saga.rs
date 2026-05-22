use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::{error, info};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::order::{CreateOrderRequest, Order, UpdateOrderParams};
use crate::models::order_state::OrderStatus;
use crate::models::role::Role;
use crate::orchestrator::SagaStep;
use crate::repositories::order_repository::OrderRepository;
use crate::services::inventory_client::InventoryClient;
use crate::services::wallet_client::WalletClient;

// SAGA CHECKOUT (worker version)

// Flow:
//   Step 1: CheckWallet          → wallet_client.check_wallet()       (read-only, no compensation)
//   Step 2: ReserveStock         → inventory_client.reserve_stock()   (kompensasi: release_stock)
//   Step 3: UpdateStatusToPending → order_repo.update() Reserving→Pending (kompensasi: set Cancelled)

pub struct CheckoutContext {
    pub titipers_id: Uuid,
    pub jastiper_id: Uuid,
    pub req: CreateOrderRequest,
    pub product: Value,
    pub unit_price: i64,
    pub service_fee: i64,
    pub total_price: i64,
    pub snapshot: Value,

    // diisi saat saga berjalan
    pub order_id: Uuid, // sudah ada dari handler (order sudah dibuat dengan status Reserving)
    pub created_order: Option<Order>, // diisi oleh UpdateStatusToPendingStep
    pub stock_reserved: bool,
}

// ── Step 1: CheckWallet ──────────────────────────────────────────────────────

pub struct CheckWalletStep {
    pub wallet_client: Arc<dyn WalletClient + Send + Sync>,
}

#[async_trait]
impl SagaStep for CheckWalletStep {
    type Context = CheckoutContext;

    async fn execute(&self, ctx: &mut CheckoutContext) -> Result<(), AppError> {
        self.wallet_client
            .check_wallet(ctx.titipers_id, ctx.total_price)
            .await
            .map_err(|e| {
                error!(
                    "❌ [CheckWalletStep] saldo tidak cukup titipers_id={} amount={}: {:?}",
                    ctx.titipers_id, ctx.total_price, e
                );
                e
            })?;

        info!(
            "✅ [CheckWalletStep] saldo cukup titipers_id={} amount={}",
            ctx.titipers_id, ctx.total_price
        );
        Ok(())
    }

    async fn compensate(&self, _ctx: &mut CheckoutContext) -> Result<(), AppError> {
        info!("↩️  [CheckWalletStep] no-op (check wallet tidak mengubah state)");
        Ok(())
    }

    fn name(&self) -> &'static str {
        "check_wallet"
    }
}

// ── Step 2: ReserveStock ─────────────────────────────────────────────────────

pub struct ReserveStockStep {
    pub inventory_client: Arc<dyn InventoryClient + Send + Sync>,
}

#[async_trait]
impl SagaStep for ReserveStockStep {
    type Context = CheckoutContext;

    async fn execute(&self, ctx: &mut CheckoutContext) -> Result<(), AppError> {
        self.inventory_client
            .reserve_stock(ctx.req.product_id, ctx.order_id, ctx.req.quantity)
            .await
            .map_err(|e| {
                error!(
                    "❌ [ReserveStockStep] reserve_stock gagal product_id={} order_id={}: {:?}",
                    ctx.req.product_id, ctx.order_id, e
                );
                e
            })?;

        ctx.stock_reserved = true;
        info!(
            "✅ [ReserveStockStep] stok berhasil direservasi product_id={} qty={} order_id={}",
            ctx.req.product_id, ctx.req.quantity, ctx.order_id
        );
        Ok(())
    }

    async fn compensate(&self, ctx: &mut CheckoutContext) -> Result<(), AppError> {
        if !ctx.stock_reserved {
            return Ok(());
        }

        error!(
            "↩️  [ReserveStockStep] melepas reservasi stok product_id={} order_id={}",
            ctx.req.product_id, ctx.order_id
        );

        self.inventory_client
            .release_stock(ctx.req.product_id, ctx.order_id, ctx.req.quantity)
            .await
            .map_err(|e| {
                error!(
                    "🚨 [ReserveStockStep] release_stock GAGAL product_id={} order_id={}: {:?}",
                    ctx.req.product_id, ctx.order_id, e
                );
                e
            })?;

        ctx.stock_reserved = false;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "reserve_stock"
    }
}

// ── Step 3: UpdateStatusToPending ────────────────────────────────────────────

pub struct UpdateStatusToPendingStep {
    pub order_repo: Arc<dyn OrderRepository + Send + Sync>,
}

#[async_trait]
impl SagaStep for UpdateStatusToPendingStep {
    type Context = CheckoutContext;

    async fn execute(&self, ctx: &mut CheckoutContext) -> Result<(), AppError> {
        let order = self
            .order_repo
            .update(
                ctx.order_id,
                &OrderStatus::Pending,
                UpdateOrderParams {
                    changed_by: "system-worker",
                    actor_role: &Role::System,
                    notes: Some("Stok berhasil direservasi, order menunggu pembayaran"),
                    tracking_number: None,
                    courier: None,
                    cancellation_reason: None,
                },
            )
            .await
            .map_err(|e| {
                error!(
                    "❌ [UpdateStatusToPendingStep] update status gagal order_id={}: {:?}",
                    ctx.order_id, e
                );
                e
            })?;

        info!(
            "✅ [UpdateStatusToPendingStep] order_id={} sekarang PENDING",
            ctx.order_id
        );
        ctx.created_order = Some(order);
        Ok(())
    }

    async fn compensate(&self, ctx: &mut CheckoutContext) -> Result<(), AppError> {
        error!(
            "↩️  [UpdateStatusToPendingStep] revert ke CANCELLED order_id={}",
            ctx.order_id
        );

        self.order_repo
            .update(
                ctx.order_id,
                &OrderStatus::Cancelled,
                UpdateOrderParams {
                    changed_by: "system-worker",
                    actor_role: &Role::System,
                    notes: Some("Checkout gagal, order dibatalkan otomatis"),
                    tracking_number: None,
                    courier: None,
                    cancellation_reason: Some("Checkout gagal saat proses reservasi"),
                },
            )
            .await
            .map_err(|e| {
                error!(
                    "🚨 [UpdateStatusToPendingStep] revert ke CANCELLED GAGAL order_id={}: {:?}",
                    ctx.order_id, e
                );
                e
            })?;

        Ok(())
    }

    fn name(&self) -> &'static str {
        "update_status_to_pending"
    }
}

// ── BUILD CONTEXT HELPER ─────────────────────────────────────────────────────

pub fn build_checkout_context(
    order_id: Uuid,
    titipers_id: Uuid,
    jastiper_id: Uuid,
    req: CreateOrderRequest,
    product: Value,
) -> CheckoutContext {
    let unit_price = product["price"].as_i64().unwrap_or(0);
    let service_fee = product["service_fee"].as_i64().unwrap_or(0);
    let total_price = (unit_price + service_fee) * req.quantity as i64;

    let snapshot = json!({
        "product_id":     req.product_id,
        "name":           product["name"],
        "description":    product["description"],
        "image_url":      product["images"][0],
        "origin_country": product["originCountry"],
        "purchase_date":  product["purchaseDate"],
        "unit_price":     unit_price,
        "service_fee":    service_fee,
    });

    CheckoutContext {
        order_id,
        titipers_id,
        jastiper_id,
        req,
        product,
        unit_price,
        service_fee,
        total_price,
        snapshot,
        created_order: None,
        stock_reserved: false,
    }
}
