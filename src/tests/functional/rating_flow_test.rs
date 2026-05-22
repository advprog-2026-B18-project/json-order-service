use crate::repositories::idempotency_repository::MockIdempotencyRepository;
use crate::repositories::order_repository::MockOrderRepository;
use crate::repositories::order_status_history_repository::MockOrderStatusHistoryRepository;
use crate::repositories::rating_jastiper_repository::MockRatingJastiperRepository;
use crate::repositories::rating_product_repository::MockRatingProductRepository;
use crate::services::inventory_client::MockInventoryClient;
use crate::services::wallet_client::MockWalletClient;
use crate::tests::unit::controller::helper_test::noop_checkout_publisher;
use crate::tests::unit::controller::helper_test::{TestApp, json_request, make_test_token};

use super::common::{make_state, setup_internal_service_key, setup_jwt_secret};

fn make_order(
    order_id: uuid::Uuid,
    user_id: uuid::Uuid,
    status: crate::models::order_state::OrderStatus,
) -> crate::models::order::Order {
    crate::models::order::Order {
        order_id,
        titipers_id: user_id,
        jastiper_id: uuid::Uuid::new_v4(),
        product_id: uuid::Uuid::new_v4(),
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

fn rating_product(
    order_id: uuid::Uuid,
    rating: f64,
    review: &str,
) -> crate::models::rating_product::RatingProduct {
    crate::models::rating_product::RatingProduct {
        rating_product_id: uuid::Uuid::new_v4(),
        order_id,
        titipers_id: uuid::Uuid::new_v4(),
        product_rating: rating,
        product_review: Some(review.to_string()),
        product_images: vec![],
        created_at: chrono::Utc::now(),
    }
}

fn rating_jastiper(
    order_id: uuid::Uuid,
    rating: f64,
    review: &str,
) -> crate::models::rating_jastiper::RatingJastiper {
    crate::models::rating_jastiper::RatingJastiper {
        rating_jastiper_id: uuid::Uuid::new_v4(),
        order_id,
        titipers_id: uuid::Uuid::new_v4(),
        jastiper_rating: rating,
        jastiper_review: Some(review.to_string()),
        created_at: chrono::Utc::now(),
    }
}

#[tokio::test]
async fn test_create_rating_product_success() {
    setup_jwt_secret();
    setup_internal_service_key();
    let user_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let order = make_order(
        order_id,
        user_id,
        crate::models::order_state::OrderStatus::Completed,
    );

    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let mut rating_repo = MockRatingProductRepository::new();
    rating_repo
        .expect_find_by_order_id()
        .returning(|_| Ok(None));
    rating_repo
        .expect_create()
        .returning(move |_, _, _| Ok(rating_product(order_id, 5.0, "Great product!")));

    let state = make_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
        rating_repo,
        MockRatingJastiperRepository::new(),
        noop_checkout_publisher(),
        MockIdempotencyRepository::new(),
    );
    let app = TestApp::new(state);
    let token = make_test_token(user_id, "buyer");
    let body = serde_json::json!({"product_rating": 5, "product_review": "Great product!", "product_images": []});
    let req = json_request(
        "POST",
        &format!("/orders/{}/rating/product", order_id),
        &token,
        Some(body),
    );
    let (status, _) = app.send(req).await;
    assert_eq!(status, axum::http::StatusCode::CREATED);
}

#[tokio::test]
async fn test_create_rating_product_order_not_found() {
    setup_jwt_secret();
    let user_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();

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
    let body = serde_json::json!({"product_rating": 5, "product_review": "Great product!", "product_images": []});
    let req = json_request(
        "POST",
        &format!("/orders/{}/rating/product", order_id),
        &token,
        Some(body),
    );
    let (status, _) = app.send(req).await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_rating_jastiper_success() {
    setup_jwt_secret();
    setup_internal_service_key();
    let user_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let order = make_order(
        order_id,
        user_id,
        crate::models::order_state::OrderStatus::Completed,
    );

    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let mut rating_repo = MockRatingJastiperRepository::new();
    rating_repo
        .expect_find_by_order_id()
        .returning(|_| Ok(None));
    rating_repo
        .expect_create()
        .returning(move |_, _, _| Ok(rating_jastiper(order_id, 4.0, "Good jastiper!")));

    let state = make_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
        MockRatingProductRepository::new(),
        rating_repo,
        noop_checkout_publisher(),
        MockIdempotencyRepository::new(),
    );
    let app = TestApp::new(state);
    let token = make_test_token(user_id, "buyer");
    let body = serde_json::json!({"jastiper_rating": 4, "jastiper_review": "Good jastiper!"});
    let req = json_request(
        "POST",
        &format!("/orders/{}/rating/jastiper", order_id),
        &token,
        Some(body),
    );
    let (status, _) = app.send(req).await;
    assert_eq!(status, axum::http::StatusCode::CREATED);
}

#[tokio::test]
async fn test_create_rating_jastiper_order_not_found() {
    setup_jwt_secret();
    let user_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();

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
    let body = serde_json::json!({"jastiper_rating": 4, "jastiper_review": "Good jastiper!"});
    let req = json_request(
        "POST",
        &format!("/orders/{}/rating/jastiper", order_id),
        &token,
        Some(body),
    );
    let (status, _) = app.send(req).await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}
