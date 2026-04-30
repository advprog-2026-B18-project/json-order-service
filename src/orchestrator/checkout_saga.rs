use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::{error, info};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::order::{CreateOrderRequest, Order, PriceBreakdown};
use crate::orchestrator::SagaStep;
use crate::ports::inventory_client::InventoryClient;
use crate::ports::order_repository::OrderRepository;
use crate::ports::wallet_client::WalletClient;

// SAGA CHECKOUT

// Flow:
//   Step 1: ReserveStock       → inventory_client.reserve_stock()
//   Step 2: CheckWallet        → wallet_client.check_wallet()      (read-only, tidak ada deduct)
//   Step 3: CreateOrder        → order_repo.create()

pub struct CheckoutContext {
    pub titipers_id: Uuid,
    pub jastiper_id: Uuid,
    pub order_id: Uuid,
    pub req: CreateOrderRequest,
    pub product: Value,
    pub unit_price: i64,
    pub service_fee: i64,
    pub total_price: i64,
    pub snapshot: Value,

    // fill while saa running
    pub stock_reserved: bool,
    pub created_order: Option<Order>,
}

pub struct ReserveStockStep {
    pub inventory_client: Arc<dyn InventoryClient>,
}

#[async_trait]
impl SagaStep for ReserveStockStep {
    type Context = CheckoutContext;

    async fn execute(&self, ctx: &mut CheckoutContext) -> Result<(), AppError> {
        self.inventory_client
            .reserve_stock(ctx.req.product_id, ctx.order_id, ctx.req.quantity)
            .await
            .map_err(|e| {
                error!("❌ [ReserveStockStep] reserve_stock gagal: {:?}", e);
                e
            })?;

        ctx.stock_reserved = true;
        info!(
            "✅ [ReserveStockStep] stok berhasil direservasi product_id={} qty={}",
            ctx.req.product_id, ctx.req.quantity
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

pub struct CheckWalletStep {
    pub wallet_client: Arc<dyn WalletClient>,
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
    pub order_repo: Arc<dyn OrderRepository>,
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

// BUILD CONTEXT HELPER
pub fn build_checkout_context(
    titipers_id: Uuid,
    jastiper_id: Uuid,
    req: CreateOrderRequest,
    product: Value,
) -> CheckoutContext {
    let order_id = Uuid::new_v4();
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
        order_id,
        req,
        product,
        unit_price,
        service_fee,
        total_price,
        snapshot,
        stock_reserved: false,
        created_order: None,
    }
}
