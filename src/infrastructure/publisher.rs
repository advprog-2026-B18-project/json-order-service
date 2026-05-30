use crate::error::AppError;
use crate::models::checkout_request::CheckoutRequest;
use async_trait::async_trait;
use deadpool_lapin::Pool;
use lapin::{
    BasicProperties, Channel, types::AMQPValue,
    options::{BasicPublishOptions, QueueDeclareOptions},
};
use tracing::log::info;

const QUEUE_NAME: &str = "checkout_requests";
const DLX_NAME: &str = "checkout_requests_dlx";

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait CheckoutPublisher: Send + Sync {
    async fn publish(&self, request: &CheckoutRequest) -> Result<(), AppError>;
}

pub struct RabbitMqCheckoutPublisher {
    pool: Pool,
    channel: tokio::sync::OnceCell<Channel>,
}

impl RabbitMqCheckoutPublisher {
    pub fn new(pool: &Pool) -> Self {
        Self {
            pool: pool.clone(),
            channel: tokio::sync::OnceCell::new(),
        }
    }

    async fn get_or_init_channel(
        &self,
    ) -> Result<&Channel, Box<dyn std::error::Error + Send + Sync>> {
        self.channel
            .get_or_try_init(|| async {
                let conn = self.pool.get().await?;
                let channel = conn.create_channel().await?;

                let dlx_args = {
                    let mut args = lapin::types::FieldTable::default();
                    args.insert(
                        "x-dead-letter-exchange".into(),
                        AMQPValue::LongString(DLX_NAME.into()),
                    );
                    args
                };

                if channel
                    .queue_declare(
                        QUEUE_NAME,
                        QueueDeclareOptions {
                            durable: true,
                            ..Default::default()
                        },
                        dlx_args,
                    )
                    .await
                    .is_err()
                {
                    tracing::warn!("[publisher] queue '{QUEUE_NAME}' sudah ada tanpa DLX, fallback");
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
                }

                Ok(channel)
            })
            .await
    }
}

#[async_trait]
impl CheckoutPublisher for RabbitMqCheckoutPublisher {
    async fn publish(&self, request: &CheckoutRequest) -> Result<(), AppError> {
        let channel = self.get_or_init_channel().await.map_err(|e| {
            tracing::error!("failed to init RabbitMQ channel: {e}");
            AppError::Internal
        })?;
        publish_checkout(channel, request).await.map_err(|e| {
            tracing::error!("checkout publish failed: {e}");
            AppError::Internal
        })
    }
}

pub async fn publish_checkout(
    channel: &Channel,
    request: &CheckoutRequest,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let payload = serde_json::to_vec(request)?;

    channel
        .basic_publish(
            "",
            QUEUE_NAME,
            BasicPublishOptions::default(),
            &payload,
            BasicProperties::default().with_delivery_mode(2),
        )
        .await?
        .await?;

    info!("{} {}", request.order_id, "published to queue");

    Ok(())
}
