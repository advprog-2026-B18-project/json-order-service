use deadpool_lapin::Pool;
use futures_lite::StreamExt;
use lapin::{
    message::Delivery,
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicQosOptions,
        QueueDeclareOptions,
    },
    types::FieldTable,
};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

use crate::error::AppError;
use crate::models::checkout_request::CheckoutRequest;
use crate::models::order::UpdateOrderParams;
use crate::models::order_state::{OrderMachine, OrderStatus};
use crate::models::role::Role;
use crate::orchestrator::SagaOrchestrator;
use crate::orchestrator::checkout_saga::{
    CheckWalletStep, ReserveStockStep, UpdateStatusToPendingStep, build_checkout_context,
};
use crate::repositories::idempotency_repository::IdempotencyRepository;
use crate::repositories::order_repository::OrderRepository;
use crate::services::inventory_client::InventoryClient;
use crate::services::wallet_client::WalletClient;

const QUEUE_NAME: &str = "checkout_requests";
const MAX_RETRY: u8 = 3;
const MAX_CONCURRENCY: usize = 10;

pub async fn run_worker(
    pool: Pool,
    order_repo: Arc<dyn OrderRepository + Send + Sync>,
    inventory_client: Arc<dyn InventoryClient + Send + Sync>,
    wallet_client: Arc<dyn WalletClient + Send + Sync>,
    idempotency_repo: Arc<dyn IdempotencyRepository + Send + Sync>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        match try_consume(
            pool.clone(),
            Arc::clone(&order_repo),
            Arc::clone(&inventory_client),
            Arc::clone(&wallet_client),
            Arc::clone(&idempotency_repo),
        )
        .await
        {
            Ok(_) => break,
            Err(e) => {
                error!("[worker] koneksi error, reconnect 5s: {e}");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }
    Ok(())
}

async fn try_consume(
    pool: Pool,
    order_repo: Arc<dyn OrderRepository + Send + Sync>,
    inventory_client: Arc<dyn InventoryClient + Send + Sync>,
    wallet_client: Arc<dyn WalletClient + Send + Sync>,
    idempotency_repo: Arc<dyn IdempotencyRepository + Send + Sync>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let conn = pool.get().await?;
    let channel = conn.create_channel().await?;

    channel
        .basic_qos(MAX_CONCURRENCY as u16, BasicQosOptions::default())
        .await?;

    channel
        .queue_declare(
            QUEUE_NAME,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    let mut consumer = channel
        .basic_consume(
            QUEUE_NAME,
            "order-service-worker",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    info!("[worker] siap consume dari '{QUEUE_NAME}' concurrency={MAX_CONCURRENCY}");

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENCY));

    while let Some(delivery) = consumer.next().await {
        let delivery = delivery?;
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("semaphore closed");

        tokio::spawn({
            let order_repo = Arc::clone(&order_repo);
            let inventory_client = Arc::clone(&inventory_client);
            let wallet_client = Arc::clone(&wallet_client);
            let idempotency_repo = Arc::clone(&idempotency_repo);

            async move {
                handle_delivery(
                    &order_repo,
                    &inventory_client,
                    &wallet_client,
                    &idempotency_repo,
                    delivery,
                )
                .await;
                drop(permit);
            }
        });
    }

    Ok(())
}

async fn handle_delivery(
    order_repo: &Arc<dyn OrderRepository + Send + Sync>,
    inventory_client: &Arc<dyn InventoryClient + Send + Sync>,
    wallet_client: &Arc<dyn WalletClient + Send + Sync>,
    idempotency_repo: &Arc<dyn IdempotencyRepository + Send + Sync>,
    delivery: Delivery,
) {
    let result = process_message(
        order_repo,
        inventory_client,
        wallet_client,
        idempotency_repo,
        &delivery,
    )
    .await;

    match result {
        Ok(_) => {
            if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
                error!("[worker] ack gagal: {e}");
            }
        }
        Err(e) => {
            error!("[worker] process_message error: {e}");

            let is_permanent =
                matches!(e, AppError::UnprocessableEntity(_) | AppError::NotFound(_));

            if is_permanent {
                warn!("[worker] permanent failure, ack untuk discard: {e}");
                let _ = delivery.ack(BasicAckOptions::default()).await;
                return;
            }

            let retry_count = delivery
                .properties
                .headers()
                .as_ref()
                .and_then(|h| h.inner().get("x-delivery-count"))
                .and_then(|v| match v {
                    lapin::types::AMQPValue::ShortShortUInt(n) => Some(*n),
                    _ => None,
                })
                .unwrap_or(0);

            if retry_count >= MAX_RETRY {
                warn!("[worker] max retry tercapai, kirim ke dead letter");
                let _ = delivery
                    .nack(BasicNackOptions {
                        requeue: false,
                        ..Default::default()
                    })
                    .await;
            } else {
                let _ = delivery
                    .nack(BasicNackOptions {
                        requeue: true,
                        ..Default::default()
                    })
                    .await;
            }
        }
    }
}

async fn process_message(
    order_repo: &Arc<dyn OrderRepository + Send + Sync>,
    inventory_client: &Arc<dyn InventoryClient + Send + Sync>,
    wallet_client: &Arc<dyn WalletClient + Send + Sync>,
    idempotency_repo: &Arc<dyn IdempotencyRepository + Send + Sync>,
    delivery: &Delivery,
) -> Result<(), AppError> {
    let request: CheckoutRequest = serde_json::from_slice(&delivery.data).map_err(|e| {
        error!("[worker] gagal deserialize message: {e}");
        AppError::Internal
    })?;

    process_checkout_request(
        order_repo,
        inventory_client,
        wallet_client,
        idempotency_repo,
        request,
    )
    .await
}

pub(crate) async fn process_checkout_request(
    order_repo: &Arc<dyn OrderRepository + Send + Sync>,
    inventory_client: &Arc<dyn InventoryClient + Send + Sync>,
    wallet_client: &Arc<dyn WalletClient + Send + Sync>,
    idempotency_repo: &Arc<dyn IdempotencyRepository + Send + Sync>,
    request: CheckoutRequest,
) -> Result<(), AppError> {
    info!(
        "[worker] processing order_id={} idempotency_key={}",
        request.order_id, request.idempotency_key
    );

    if idempotency_repo
        .is_processed(request.idempotency_key)
        .await?
    {
        warn!(
            "[worker] duplicate message order_id={}, skip",
            request.order_id
        );
        return Ok(());
    }

    let order = order_repo
        .find_by_id(request.order_id)
        .await?
        .ok_or_else(|| {
            error!("[worker] order_id={} tidak ditemukan", request.order_id);
            AppError::NotFound("Order tidak ditemukan".to_string())
        })?;

    if order.status != OrderStatus::Reserving {
        warn!(
            "[worker] order_id={} sudah bukan Reserving (status={}), skip",
            request.order_id, order.status
        );
        idempotency_repo
            .mark_processed(request.idempotency_key, request.order_id)
            .await?;
        return Ok(());
    }

    let mut ctx = build_checkout_context(
        request.order_id,
        request.titipers_id,
        request.jastiper_id,
        request.req,
        request.product,
    );

    let saga = SagaOrchestrator::new("checkout_worker")
        .step(CheckWalletStep {
            wallet_client: Arc::clone(wallet_client),
        })
        .step(ReserveStockStep {
            inventory_client: Arc::clone(inventory_client),
        })
        .step(UpdateStatusToPendingStep {
            order_repo: Arc::clone(order_repo),
        });

    match saga.run(&mut ctx).await {
        Ok(_) => {
            idempotency_repo
                .mark_processed(request.idempotency_key, request.order_id)
                .await?;
            info!(
                "[worker] checkout selesai order_id={} sekarang PENDING",
                request.order_id
            );
            Ok(())
        }
        Err(e) => {
            match order_repo.find_by_id(ctx.order_id).await {
                Ok(Some(order)) => {
                    let machine = OrderMachine::from_status(&order.status);
                    match machine.cancel(&Role::System) {
                        Ok(new_status) => {
                            if let Err(cancel_err) = order_repo
                                .update(
                                    ctx.order_id,
                                    &new_status,
                                    UpdateOrderParams {
                                        changed_by: "system",
                                        actor_role: &Role::System,
                                        notes: Some("Checkout gagal, order dibatalkan otomatis"),
                                        tracking_number: None,
                                        courier: None,
                                        cancellation_reason: Some(&e.to_string()),
                                    },
                                )
                                .await
                            {
                                error!(
                                    "[worker] gagal cancel order_id={}: {cancel_err}",
                                    ctx.order_id
                                );
                            } else {
                                info!("[worker] order_id={} di-set Cancelled", ctx.order_id);
                            }
                        }
                        Err(transition_err) => {
                            error!(
                                "[worker] transisi status tidak valid order_id={}: {transition_err}",
                                ctx.order_id
                            );
                        }
                    }
                }
                Ok(None) => {
                    error!(
                        "[worker] order_id={} tidak ditemukan saat cancel",
                        ctx.order_id
                    );
                }
                Err(db_err) => {
                    error!(
                        "[worker] gagal fetch order saat cancel order_id={}: {db_err}",
                        ctx.order_id
                    );
                }
            }

            if let Err(idm_err) = idempotency_repo
                .mark_processed(request.idempotency_key, request.order_id)
                .await
            {
                error!("[worker] gagal mark idempotency setelah saga fail: {idm_err}");
            }

            Err(e)
        }
    }
}
