use uuid::Uuid;

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

#[tokio::test]
async fn test_admin_get_all_orders_as_non_admin_returns_forbidden() {
    setup_jwt_secret();
    let user_id = Uuid::new_v4();

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

    let token = make_test_token(user_id, "TITIPERS");
    let req = json_request("GET", "/admin/orders", &token, None);
    let (status, _) = app.send(req).await;
    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_admin_get_order_as_non_admin_returns_forbidden() {
    setup_jwt_secret();
    let user_id = Uuid::new_v4();
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

    let token = make_test_token(user_id, "TITIPERS");
    let req = json_request("GET", &format!("/admin/orders/{}", order_id), &token, None);
    let (status, _) = app.send(req).await;
    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
}
