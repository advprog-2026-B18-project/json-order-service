use uuid::Uuid;

use crate::infrastructure::publisher::MockCheckoutPublisher;
use crate::models::order::Order;
use crate::models::order_state::OrderStatus;
use crate::repositories::idempotency_repository::MockIdempotencyRepository;
use crate::repositories::order_repository::MockOrderRepository;
use crate::repositories::order_status_history_repository::MockOrderStatusHistoryRepository;
use crate::repositories::rating_jastiper_repository::MockRatingJastiperRepository;
use crate::repositories::rating_product_repository::MockRatingProductRepository;
use crate::services::inventory_client::MockInventoryClient;
use crate::services::wallet_client::MockWalletClient;
use crate::tests::unit::controller::helper_test::noop_checkout_publisher;
use crate::tests::unit::controller::helper_test::{TestApp, json_request, make_test_token};

use super::common::{make_state, setup_jwt_secret};

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
async fn test_checkout_without_auth_returns_unauthorized() {
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
        .method("POST")
        .uri("/orders")
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            r#"{"product_id":"00000000-0000-0000-0000-000000000000","quantity":1,"shipping_address":{"recipient_name":"Test","phone_number":"123","street":"Jln","kelurahan":"A","kecamatan":"B","city":"C","province":"D","postal_code":"12345"},"note_to_jastiper":null}"#,
        ))
        .expect("build req");
    let (status, _) = app.send(req).await;
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_checkout_product_not_found() {
    setup_jwt_secret();
    let user_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let order = make_order(Uuid::new_v4(), user_id, OrderStatus::Pending);

    let mut repo = MockOrderRepository::new();
    repo.expect_create()
        .returning(move |_, _, _, _, _| Ok(order.clone()));
    repo.expect_find_by_id().returning(|_| Ok(None));

    let mut inv = MockInventoryClient::new();
    inv.expect_fetch_product().returning(move |_| {
        Ok(serde_json::json!({
            "product_id": product_id,
            "name": "Test Item",
            "price": 10000,
            "service_fee": 1000,
            "description": "desc",
            "images": [],
            "originCountry": "ID",
            "purchaseDate": "2026-01-01",
            "jastiper": { "user_id": Uuid::new_v4().to_string() },
        }))
    });
    inv.expect_reserve_stock().returning(|_, _, _| Ok(()));

    let state = make_state(
        repo,
        inv,
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
        MockRatingProductRepository::new(),
        MockRatingJastiperRepository::new(),
        noop_checkout_publisher(),
        MockIdempotencyRepository::new(),
    );
    let app = TestApp::new(state);

    let token = make_test_token(user_id, "buyer");
    let body = serde_json::json!({
        "product_id": product_id.to_string(),
        "quantity": 1,
        "shipping_address": {
            "recipient_name": "Test",
            "phone_number": "123",
            "street": "Jln",
            "kelurahan": "A",
            "kecamatan": "B",
            "city": "C",
            "province": "D",
            "postal_code": "12345"
        },
        "note_to_jastiper": null
    });
    let req = json_request("POST", "/orders", &token, Some(body));
    let (status, _) = app.send(req).await;
    assert_eq!(status, axum::http::StatusCode::ACCEPTED);
}

#[tokio::test]
async fn test_checkout_publisher_failure_returns_500() {
    setup_jwt_secret();
    let user_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let order = make_order(Uuid::new_v4(), user_id, OrderStatus::Pending);

    let mut repo = MockOrderRepository::new();
    repo.expect_create()
        .returning(move |_, _, _, _, _| Ok(order.clone()));
    repo.expect_find_by_id().returning(|_| Ok(None));

    let mut inv = MockInventoryClient::new();
    inv.expect_fetch_product().returning(move |_| {
        Ok(serde_json::json!({
            "product_id": product_id,
            "name": "Test",
            "price": 10000,
            "service_fee": 1000,
            "description": "desc",
            "images": [],
            "originCountry": "ID",
            "purchaseDate": "2026-01-01",
            "jastiper": { "user_id": Uuid::new_v4().to_string() },
        }))
    });
    inv.expect_reserve_stock().returning(|_, _, _| Ok(()));

    let mut publisher = MockCheckoutPublisher::new();
    publisher
        .expect_publish()
        .returning(|_| Err(crate::error::AppError::Internal));

    let state = make_state(
        repo,
        inv,
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
        MockRatingProductRepository::new(),
        MockRatingJastiperRepository::new(),
        publisher,
        MockIdempotencyRepository::new(),
    );
    let app = TestApp::new(state);

    let token = make_test_token(user_id, "buyer");
    let body = serde_json::json!({
        "product_id": product_id.to_string(),
        "quantity": 1,
        "shipping_address": {
            "recipient_name": "Test",
            "phone_number": "123",
            "street": "Jln",
            "kelurahan": "A",
            "kecamatan": "B",
            "city": "C",
            "province": "D",
            "postal_code": "12345"
        },
        "note_to_jastiper": null
    });
    let req = json_request("POST", "/orders", &token, Some(body));
    let (status, _) = app.send(req).await;
    assert_eq!(status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_get_order_success() {
    setup_jwt_secret();
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

    let token = make_test_token(user_id, "buyer");
    let req = json_request("GET", &format!("/orders/{}", order_id), &token, None);
    let (status, body) = app.send(req).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(
        body["data"]["order_id"],
        serde_json::json!(order_id.to_string())
    );
}

#[tokio::test]
async fn test_get_order_not_found() {
    setup_jwt_secret();
    let user_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

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

    let token = make_test_token(user_id, "buyer");
    let req = json_request("GET", &format!("/orders/{}", order_id), &token, None);
    let (status, _) = app.send(req).await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_cancel_order_success() {
    setup_jwt_secret();
    let user_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let order = make_order(order_id, user_id, OrderStatus::Pending);

    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(make_order(order_id, user_id, OrderStatus::Cancelled)));

    let mut inv = MockInventoryClient::new();
    inv.expect_release_stock().returning(|_, _, _| Ok(()));

    let state = make_state(
        repo,
        inv,
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
        MockRatingProductRepository::new(),
        MockRatingJastiperRepository::new(),
        noop_checkout_publisher(),
        MockIdempotencyRepository::new(),
    );
    let app = TestApp::new(state);

    let token = make_test_token(user_id, "ADMIN");
    let body = serde_json::json!({
        "cancellation_reason": "User requested cancellation"
    });
    let req = json_request(
        "POST",
        &format!("/orders/{}/cancel", order_id),
        &token,
        Some(body),
    );
    let (status, _) = app.send(req).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[tokio::test]
async fn test_confirm_order_success() {
    setup_jwt_secret();
    let user_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let order = make_order(order_id, user_id, OrderStatus::Shipped);

    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(make_order(order_id, user_id, OrderStatus::Completed)));

    let mut wallet = MockWalletClient::new();
    wallet.expect_earnings_wallet().returning(|_, _, _| {
        Ok(crate::services::wallet_client::EarningsResponse {
            transaction_id: "tx-1".to_string(),
        })
    });

    let mut inv = MockInventoryClient::new();
    inv.expect_confirm_order_received().returning(|_, _| Ok(()));

    let state = make_state(
        repo,
        inv,
        wallet,
        MockOrderStatusHistoryRepository::new(),
        MockRatingProductRepository::new(),
        MockRatingJastiperRepository::new(),
        noop_checkout_publisher(),
        MockIdempotencyRepository::new(),
    );
    let app = TestApp::new(state);

    let token = make_test_token(user_id, "TITIPERS");
    let req = json_request(
        "PATCH",
        &format!("/orders/{}/confirm", order_id),
        &token,
        None,
    );
    let (status, _) = app.send(req).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[tokio::test]
async fn test_get_orders_by_user_returns_list() {
    setup_jwt_secret();
    let user_id = Uuid::new_v4();
    let order = make_order(Uuid::new_v4(), user_id, OrderStatus::Pending);

    let mut repo = MockOrderRepository::new();
    repo.expect_find_all()
        .returning(move |_, _| Ok((vec![order.clone()], 1i64)));

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

    let token = make_test_token(user_id, "buyer");
    let req = json_request("GET", "/orders/my/purchases", &token, None);
    let (status, body) = app.send(req).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(body["data"].is_array());
}
