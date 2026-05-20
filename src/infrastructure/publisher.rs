use crate::models::checkout_request::CheckoutRequest;
use deadpool_lapin::Pool;
use lapin::{
    BasicProperties,
    options::{BasicPublishOptions, QueueDeclareOptions},
};
use tracing::log::info;

const QUEUE_NAME: &str = "checkout_requests";

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

    info!("{} {}", request.order_id.to_string(), "published to queue");

    Ok(())
}
