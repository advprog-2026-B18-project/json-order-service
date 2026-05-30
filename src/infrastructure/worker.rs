use deadpool_lapin::Pool;
use futures_lite::StreamExt;
use lapin::{
    message::Delivery,
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicQosOptions,
        ExchangeDeclareOptions, QueueDeclareOptions,
    },
    types::{AMQPValue, FieldTable},
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
use crate::services::auth_client::AuthClient;
use crate::services::inventory_client::InventoryClient;
use crate::services::wallet_client::WalletClient;

const QUEUE_NAME: &str = "checkout_requests";
const DLX_NAME: &str = "checkout_requests_dlx";
const DLQ_NAME: &str = "checkout_requests_dlq";
const MAX_RETRY: u8 = 3;
const MAX_CONCURRENCY: usize = 10;

fn retry_count_from_delivery(delivery: &Delivery) -> u8 {
    let headers = match delivery.properties.headers().as_ref() {
        Some(h) => h,
        None => return 0,
    };

    if let Some(AMQPValue::ShortShortUInt(n)) = headers.inner().get("x-delivery-count") {
        return *n;
    }

    if let Some(AMQPValue::FieldArray(arr)) = headers.inner().get("x-death") {
        for entry in arr.as_slice() {
            if let AMQPValue::FieldTable(table) = entry {
                if let Some(AMQPValue::LongUInt(n)) = table.inner().get("count") {
                    return *n as u8;
                }
            }
        }
    }

    0
}

fn is_permanent(e: &AppError) -> bool {
    matches!(
        e,
        AppError::Validation(_)
            | AppError::Unauthorized(_)
            | AppError::Forbidden(_)
            | AppError::NotFound(_)
            | AppError::Conflict(_)
            | AppError::UnprocessableEntity(_)
            | AppError::InvalidStatusTransition { .. }
            | AppError::InsufficientBalance
            | AppError::LimitExceeded
    )
}

pub async fn run_worker(
    pool: Pool,
    order_repo: Arc<dyn OrderRepository + Send + Sync>,
    inventory_client: Arc<dyn InventoryClient + Send + Sync>,
    wallet_client: Arc<dyn WalletClient + Send + Sync>,
    auth_client: Arc<dyn AuthClient + Send + Sync>,
    idempotency_repo: Arc<dyn IdempotencyRepository + Send + Sync>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        match try_consume(
            pool.clone(),
            Arc::clone(&order_repo),
            Arc::clone(&inventory_client),
            Arc::clone(&wallet_client),
            Arc::clone(&auth_client),
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
    auth_client: Arc<dyn AuthClient + Send + Sync>,
    idempotency_repo: Arc<dyn IdempotencyRepository + Send + Sync>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let conn = pool.get().await?;
    let channel = conn.create_channel().await?;

    channel
        .basic_qos(MAX_CONCURRENCY as u16, BasicQosOptions::default())
        .await?;

    channel
        .exchange_declare(
            DLX_NAME,
            lapin::ExchangeKind::Fanout,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_declare(
            DLQ_NAME,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_bind(
            DLQ_NAME,
            DLX_NAME,
            "",
            Default::default(),
            FieldTable::default(),
        )
        .await?;

    let queue_declare_with_dlx = || async {
        let mut args = FieldTable::default();
        args.insert(
            "x-dead-letter-exchange".into(),
            AMQPValue::LongString(DLX_NAME.into()),
        );
        channel
            .queue_declare(
                QUEUE_NAME,
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                args,
            )
            .await
    };

    match queue_declare_with_dlx().await {
        Ok(_) => {
            info!("[worker] queue '{}' dengan DLX siap", QUEUE_NAME);
        }
        Err(e) => {
            warn!(
                "[worker] queue '{}' sudah ada tanpa DLX, fallback: {e}",
                QUEUE_NAME
            );
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
        }
    }

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
            let auth_client = Arc::clone(&auth_client);
            let idempotency_repo = Arc::clone(&idempotency_repo);

            async move {
                handle_delivery(
                    &order_repo,
                    &inventory_client,
                    &wallet_client,
                    &auth_client,
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
    auth_client: &Arc<dyn AuthClient + Send + Sync>,
    idempotency_repo: &Arc<dyn IdempotencyRepository + Send + Sync>,
    delivery: Delivery,
) {
    let result = process_message(
        order_repo,
        inventory_client,
        wallet_client,
        auth_client,
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

            if is_permanent(&e) {
                warn!("[worker] permanent error, cancel + ack: {e}");
                if let Ok(request) = serde_json::from_slice::<CheckoutRequest>(&delivery.data) {
                    cancel_and_mark_processed(order_repo, idempotency_repo, &request).await;
                }
                let _ = delivery.ack(BasicAckOptions::default()).await;
                return;
            }

            let retry_count = retry_count_from_delivery(&delivery);

            if retry_count >= MAX_RETRY {
                warn!("[worker] max retry tercapai, cancel + dead letter: {e}");
                if let Ok(request) = serde_json::from_slice::<CheckoutRequest>(&delivery.data) {
                    cancel_and_mark_processed(order_repo, idempotency_repo, &request).await;
                }
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

async fn cancel_and_mark_processed(
    order_repo: &Arc<dyn OrderRepository + Send + Sync>,
    idempotency_repo: &Arc<dyn IdempotencyRepository + Send + Sync>,
    request: &CheckoutRequest,
) {
    if let Ok(Some(order)) = order_repo.find_by_id(request.order_id).await {
        let machine = OrderMachine::from_status(&order.status);
        if let Ok(new_status) = machine.cancel(&Role::System) {
            if let Err(e) = order_repo
                .update(
                    request.order_id,
                    &new_status,
                    UpdateOrderParams {
                        changed_by: "system",
                        actor_role: &Role::System,
                        notes: Some("Checkout gagal, order dibatalkan otomatis"),
                        tracking_number: None,
                        courier: None,
                        cancellation_reason: Some("Checkout gagal saat proses worker"),
                    },
                )
                .await
            {
                error!("[worker] gagal cancel order_id={}: {e}", request.order_id);
            } else {
                info!("[worker] order_id={} di-set Cancelled", request.order_id);
            }
        }
    }

    if let Err(e) = idempotency_repo
        .mark_processed(request.idempotency_key, request.order_id)
        .await
    {
        error!("[worker] gagal mark idempotent order_id={}: {e}", request.order_id);
    }
}

async fn process_message(
    order_repo: &Arc<dyn OrderRepository + Send + Sync>,
    inventory_client: &Arc<dyn InventoryClient + Send + Sync>,
    wallet_client: &Arc<dyn WalletClient + Send + Sync>,
    auth_client: &Arc<dyn AuthClient + Send + Sync>,
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
        auth_client,
        idempotency_repo,
        request,
    )
    .await
}

pub(crate) async fn process_checkout_request(
    order_repo: &Arc<dyn OrderRepository + Send + Sync>,
    inventory_client: &Arc<dyn InventoryClient + Send + Sync>,
    wallet_client: &Arc<dyn WalletClient + Send + Sync>,
    auth_client: &Arc<dyn AuthClient + Send + Sync>,
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

            // Notify auth service that this jastiper has a new order
            // TODO 1

            info!(
                "[worker] checkout selesai order_id={} sekarang PENDING",
                request.order_id
            );
            Ok(())
        }
        Err(e) => {
            // Order cancellation + idempotency marking is handled in handle_delivery
            // based on ErrorClass classification (fatal vs transient).
            // For transient errors: no side effects, retry re-runs the saga.
            // For fatal/max-retry errors: handle_delivery calls cancel_and_mark_processed.
            Err(e)
        }
    }
}
