use uuid::Uuid;

use crate::models::order::Order;
use crate::models::order_state::OrderStatus;
use crate::repositories::idempotency_repository::MockIdempotencyRepository;
use crate::repositories::order_repository::MockOrderRepository;
use crate::repositories::order_status_history_repository::MockOrderStatusHistoryRepository;
use crate::repositories::rating_jastiper_repository::MockRatingJastiperRepository;
use crate::repositories::rating_product_repository::MockRatingProductRepository;
use crate::services::inventory_client::MockInventoryClient;
use crate::services::wallet_client::MockWalletClient;
use crate::tests::unit::controller::helper_test::TestApp;
use crate::tests::unit::controller::helper_test::noop_checkout_publisher;

use super::common::make_state;

fn internal_request(
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
) -> axum::http::Request<axum::body::Body> {
    let body_bytes = body.map(|v| v.to_string().into_bytes()).unwrap_or_default();
    let service_key =
        std::env::var("INTERNAL_SERVICE_KEY").unwrap_or_else(|_| "internal-secret".to_string());
    axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .header("Content-Type", "application/json")
        .header("X-Service-Key", &service_key)
        .body(axum::body::Body::from(body_bytes))
        .expect("build internal request")
}

fn make_order(order_id: Uuid, user_id: Uuid, status: OrderStatus) -> Order {
    Order {
        order_id,
        titipers_id: user_id,
        jastiper_id: Uuid::new_v4(),
        product_id: Uuid::new_v4(),
        product_snapshot: serde_json::json!({}),
        quantity: 1,
        unit_price: 10000,
        service_fee: 1000,
        total_price: 11000,
        status,
        shipping_address: serde_json::json!({}),
        note_to_jastiper: None,
        tracking_number: None,
        courier: None,
        cancellation_reason: None,
        cancelled_by: None,
        completed_at: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        expired_at: chrono::Utc::now(),
    }
}

#[tokio::test]
async fn test_internal_get_payment_info_success() {
    let user_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let order = make_order(order_id, user_id, OrderStatus::Pending);

    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let state = make_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
        MockRatingProductRepository::new(),
        MockRatingJastiperRepository::new(),
        noop_checkout_publisher(),
        MockIdempotencyRepository::new(),
    );
    let app = TestApp::new(state);

    let req = internal_request(
        "GET",
        &format!("/internal/orders/{}/payment-info", order_id),
        None,
    );
    let (status, body) = app.send(req).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(body["data"]["total_price"].is_number());
}

#[tokio::test]
async fn test_internal_get_payment_info_requires_service_key() {
    let order_id = Uuid::new_v4();

    let state = make_state(
        MockOrderRepository::new(),
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
        MockRatingProductRepository::new(),
        MockRatingJastiperRepository::new(),
        noop_checkout_publisher(),
        MockIdempotencyRepository::new(),
    );
    let app = TestApp::new(state);

    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/internal/orders/{}/payment-info", order_id))
        .body(axum::body::Body::empty())
        .expect("build req");
    let (status, _) = app.send(req).await;
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_internal_refund_confirmed_success() {
    let user_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let order = make_order(order_id, user_id, OrderStatus::Refunding);

    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(make_order(order_id, user_id, OrderStatus::Refunding)));

    let state = make_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
        MockRatingProductRepository::new(),
        MockRatingJastiperRepository::new(),
        noop_checkout_publisher(),
        MockIdempotencyRepository::new(),
    );
    let app = TestApp::new(state);

    let body = serde_json::json!({
        "success": true,
        "wallet_transaction_id": Uuid::new_v4().to_string(),
        "amount_refunded": 11000,
        "notes": null,
    });
    let req = internal_request(
        "POST",
        &format!("/internal/orders/{}/refund-confirmed", order_id),
        Some(body),
    );
    let (status, _) = app.send(req).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}
