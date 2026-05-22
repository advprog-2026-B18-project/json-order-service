use crate::error::AppError;
use crate::models::checkout_request::CheckoutRequest;
use async_trait::async_trait;
use deadpool_lapin::Pool;
use lapin::{
    BasicProperties,
    options::{BasicPublishOptions, QueueDeclareOptions},
};
use tracing::log::info;

const QUEUE_NAME: &str = "checkout_requests";

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait CheckoutPublisher: Send + Sync {
    async fn publish(&self, request: &CheckoutRequest) -> Result<(), AppError>;
}

pub struct RabbitMqCheckoutPublisher {
    pool: Pool,
}

impl RabbitMqCheckoutPublisher {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CheckoutPublisher for RabbitMqCheckoutPublisher {
    async fn publish(&self, request: &CheckoutRequest) -> Result<(), AppError> {
        publish_checkout(&self.pool, request).await.map_err(|e| {
            tracing::error!("checkout publish failed: {e}");
            AppError::Internal
        })
    }
}

pub async fn publish_checkout(
    pool: &Pool,
    request: &CheckoutRequest,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let conn = pool.get().await?;
    let channel = conn.create_channel().await?;

    channel
        .queue_declare(
            QUEUE_NAME,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            Default::default(),
        )
        .await?;

    let payload = serde_json::to_vec(request)?;

    channel
        .basic_publish(
            "",
            QUEUE_NAME,
            BasicPublishOptions::default(),
            &payload,
            BasicProperties::default().with_delivery_mode(2), // persistent
        )
        .await?
        .await?;

    info!("{} {}", request.order_id, "published to queue");

    Ok(())
}
