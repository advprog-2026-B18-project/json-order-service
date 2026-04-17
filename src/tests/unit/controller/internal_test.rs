use axum::http::StatusCode;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::order::{Order, OrderStatus};
use crate::ports::auth_client::MockAuthClient;
use crate::ports::inventory_client::MockInventoryClient;
use crate::ports::order_repository::MockOrderRepository;
use crate::ports::order_status_history_repository::MockOrderStatusHistoryRepository;
use crate::ports::rating_jastiper_repository::MockRatingJastiperRepository;
use crate::ports::rating_product_repository::MockRatingProductRepository;
use crate::ports::wallet_client::MockWalletClient;
use crate::state::AppState;
use crate::tests::unit::controller::helper_test::{
    TestApp, json_request_internal, json_request_internal_post,
};

pub fn setup_jwt_secret() {
    unsafe {
        std::env::set_var("JWT_SECRET", "dGVzdC1zZWNyZXQtdGVzdC1zZWNyZXQ=");
    }
}

pub fn setup_service_key() {
    unsafe {
        std::env::set_var("SERVICE_KEY", "valid-service-key-123");
    }
}

fn make_order(order_id: Uuid, titipers_id: Uuid, jastiper_id: Uuid, status: OrderStatus) -> Order {
    Order {
        order_id,
        titipers_id,
        jastiper_id,
        product_id: Uuid::new_v4(),
        product_snapshot: json!({}),
        quantity: 1,
        unit_price: 10_000,
        service_fee: 1_000,
        total_price: 11_000,
        status,
        shipping_address: json!({}),
        note_to_jastiper: None,
        tracking_number: None,
        courier: None,
        cancellation_reason: None,
        cancelled_by: None,
        completed_at: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

#[tokio::test]
async fn payment_info_gagal_service_key_invalid_401() {
    setup_jwt_secret();
    setup_service_key();

    let order_id = Uuid::new_v4();

    let app = TestApp::new(AppState {
        order_repo: Arc::new(MockOrderRepository::new()),
        inventory_client: Arc::new(MockInventoryClient::new()),
        wallet_client: Arc::new(MockWalletClient::new()),
        order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
        rating_product_repo: Arc::new(MockRatingProductRepository::new()),
        rating_jastiper_repo: Arc::new(MockRatingJastiperRepository::new()),
        auth_client: Arc::new(MockAuthClient::new()),
    });

    let req = json_request_internal(
        "GET",
        &format!("/internal/orders/{}/payment-info", order_id),
        "invalid-key",
    );

    let (status, _) = app.send(req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refund_confirmed_gagal_amount_mismatch_422() {
    setup_jwt_secret();
    setup_service_key();

    let order_id = Uuid::new_v4();
    let mut repo = MockOrderRepository::new();

    let refunding = make_order(
        order_id,
        Uuid::new_v4(),
        Uuid::new_v4(),
        OrderStatus::Refunding,
    );

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(refunding.clone())));

    let app = TestApp::new(AppState {
        order_repo: Arc::new(repo),
        inventory_client: Arc::new(MockInventoryClient::new()),
        wallet_client: Arc::new(MockWalletClient::new()),
        order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
        rating_product_repo: Arc::new(MockRatingProductRepository::new()),
        rating_jastiper_repo: Arc::new(MockRatingJastiperRepository::new()),
        auth_client: Arc::new(MockAuthClient::new()),
    });

    let req = json_request_internal_post(
        &format!("/internal/orders/{}/refund-confirmed", order_id),
        "super-secret-internal-key-2026",
        Some(json!({
            "success": true,
            "amount_refunded": 9999,
        })),
    );

    let (status, _) = app.send(req).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
