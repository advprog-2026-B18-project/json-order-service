use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::{error, info};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::order::{CreateOrderRequest, Order, PriceBreakdown};
use crate::orchestrator::SagaStep;
use crate::repositories::order_repository::OrderRepository;
use crate::services::inventory_client::InventoryClient;
use crate::services::wallet_client::WalletClient;

// SAGA CHECKOUT

// Flow:
//   Step 1: CheckWallet   → wallet_client.check_wallet()      (read-only, no compensation)
//   Step 2: CreateOrder   → order_repo.create()               (UUID dari repo)
//   Step 3: ReserveStock  → inventory_client.reserve_stock()  (pakai order_id dari ctx.created_order)

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
    pub created_order: Option<Order>,
    pub stock_reserved: bool,
}

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

pub struct CreateOrderStep {
    pub order_repo: Arc<dyn OrderRepository + Send + Sync>,
}

#[async_trait]
impl SagaStep for CreateOrderStep {
    type Context = CheckoutContext;

    async fn execute(&self, ctx: &mut CheckoutContext) -> Result<(), AppError> {
        let order = self
            .order_repo
            .create(
                ctx.titipers_id,
                ctx.jastiper_id,
                ctx.req.clone(),
                ctx.snapshot.clone(),
                PriceBreakdown {
                    unit_price: ctx.unit_price,
                    service_fee: ctx.service_fee,
                    total_price: ctx.total_price,
                },
            )
            .await
            .map_err(|e| {
                error!("❌ [CreateOrderStep] create order gagal: {:?}", e);
                e
            })?;

        info!(
            "✅ [CreateOrderStep] order dibuat order_id={}",
            order.order_id
        );
        ctx.created_order = Some(order);
        Ok(())
    }

    async fn compensate(&self, ctx: &mut CheckoutContext) -> Result<(), AppError> {
        if let Some(order) = &ctx.created_order {
            error!(
                "↩️  [CreateOrderStep] membatalkan order order_id={}",
                order.order_id
            );
            self.order_repo.delete(order.order_id).await.map_err(|e| {
                error!("🚨 [CreateOrderStep] delete order GAGAL: {:?}", e);
                e
            })?;
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "create_order"
    }
}

pub struct ReserveStockStep {
    pub inventory_client: Arc<dyn InventoryClient + Send + Sync>,
}

#[async_trait]
impl SagaStep for ReserveStockStep {
    type Context = CheckoutContext;

    async fn execute(&self, ctx: &mut CheckoutContext) -> Result<(), AppError> {
        let order_id = ctx
            .created_order
            .as_ref()
            .ok_or_else(|| {
                error!("❌ [ReserveStockStep] created_order belum ada di context");
                AppError::Internal
            })?
            .order_id;

        self.inventory_client
            .reserve_stock(ctx.req.product_id, order_id, ctx.req.quantity)
            .await
            .map_err(|e| {
                error!("❌ [ReserveStockStep] reserve_stock gagal: {:?}", e);
                e
            })?;

        ctx.stock_reserved = true;
        info!(
            "✅ [ReserveStockStep] stok berhasil direservasi product_id={} qty={} order_id={}",
            ctx.req.product_id, ctx.req.quantity, order_id
        );
        Ok(())
    }

    async fn compensate(&self, ctx: &mut CheckoutContext) -> Result<(), AppError> {
        if !ctx.stock_reserved {
            return Ok(());
        }

        let order_id = match ctx.created_order.as_ref() {
            Some(o) => o.order_id,
            None => {
                error!("↩️  [ReserveStockStep] tidak bisa release — created_order tidak ada");
                return Ok(());
            }
        };

        error!(
            "↩️  [ReserveStockStep] melepas reservasi stok product_id={} order_id={}",
            ctx.req.product_id, order_id
        );
        self.inventory_client
            .release_stock(ctx.req.product_id, order_id, ctx.req.quantity)
            .await
            .map_err(|e| {
                error!("🚨 [ReserveStockStep] release_stock GAGAL: {:?}", e);
                e
            })?;

        ctx.stock_reserved = false;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "reserve_stock"
    }
}

// BUILD CONTEXT HELPER
pub fn build_checkout_context(
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
