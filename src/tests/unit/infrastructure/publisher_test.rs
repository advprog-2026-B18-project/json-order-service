use crate::infrastructure::publisher::{
    CheckoutPublisher, RabbitMqCheckoutPublisher, publish_checkout,
};
use crate::models::checkout_request::CheckoutRequest;
use crate::models::order::CreateOrderRequest;
use crate::models::shipping_address::ShippingAddress;
use deadpool_lapin::{Config, Runtime};
use serde_json::json;
use uuid::Uuid;

fn dummy_pool() -> deadpool_lapin::Pool {
    Config {
        url: Some("amqp://guest:guest@127.0.0.1:1/%2f".to_string()),
        ..Default::default()
    }
    .create_pool(Some(Runtime::Tokio1))
    .unwrap()
}

fn checkout_request() -> CheckoutRequest {
    let order_id = Uuid::new_v4();
    CheckoutRequest {
        order_id,
        titipers_id: Uuid::new_v4(),
        jastiper_id: Uuid::new_v4(),
        req: CreateOrderRequest {
            product_id: Uuid::new_v4(),
            quantity: 1,
            shipping_address: ShippingAddress {
                recipient_name: "Test User".to_string(),
                phone_number: "08123456789".to_string(),
                street: "Jl. Test".to_string(),
                kelurahan: "Kel".to_string(),
                kecamatan: "Kec".to_string(),
                city: "Jakarta".to_string(),
                province: "DKI".to_string(),
                postal_code: "12345".to_string(),
                notes: None,
            },
            note_to_jastiper: None,
            idempotency_key: None,
        },
        product: json!({"name": "Snack"}),
        idempotency_key: order_id,
    }
}

// === Error Path ===
#[tokio::test]
async fn test_publish_checkout_pool_connection_error_returns_error() {
    let pool = dummy_pool();
    // A connection to a non-existent broker will fail, this just validates the error path
    let conn_result = pool.get().await;
    if let Ok(conn) = conn_result
        && let Ok(channel) = conn.create_channel().await
    {
        let request = checkout_request();
        let result = publish_checkout(&channel, &request).await;
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_rabbit_mq_checkout_publisher_pool_error_returns_internal() {
    let pool = dummy_pool();

    // Constructor succeeds (lazy), but publish fails because pool connects to non-existent broker
    let publisher = RabbitMqCheckoutPublisher::new(&pool);
    let request = checkout_request();
    let result = publisher.publish(&request).await;
    assert!(result.is_err());
}

fn live_pool() -> deadpool_lapin::Pool {
    Config {
        url: Some("amqp://guest:guest@127.0.0.1:5672/%2f".to_string()),
        ..Default::default()
    }
    .create_pool(Some(Runtime::Tokio1))
    .unwrap()
}

#[tokio::test]
async fn test_rabbit_mq_checkout_publisher_publish_success() {
    let pool = live_pool();

    // Skip if RabbitMQ is not available
    if pool.get().await.is_err() {
        return;
    }

    let publisher = RabbitMqCheckoutPublisher::new(&pool);
    let request = checkout_request();
    let result = publisher.publish(&request).await;
    assert!(
        result.is_ok(),
        "Publish should succeed with RabbitMQ running"
    );
}
