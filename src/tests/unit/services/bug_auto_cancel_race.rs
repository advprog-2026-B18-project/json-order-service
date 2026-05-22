use std::sync::Arc;
use uuid::Uuid;

use crate::models::order::{CancelRequest, Order};
use crate::models::order_state::OrderStatus;
use crate::models::role::Role;
use crate::repositories::order_repository::MockOrderRepository;
use crate::services::inventory_client::MockInventoryClient;
use crate::services::order::cancel_order;
use crate::services::wallet_client::MockWalletClient;

use crate::tests::functional::common::setup_jwt_secret;

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
async fn test_auto_cancel_does_not_affect_completed_orders() {
    setup_jwt_secret();
    let order_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(make_order(order_id, user_id, OrderStatus::Completed))));

    let order_repo: Arc<dyn crate::repositories::order_repository::OrderRepository + Send + Sync> =
        Arc::new(order_repo);
    let inv_client: Arc<dyn crate::services::inventory_client::InventoryClient + Send + Sync> =
        Arc::new(MockInventoryClient::new());
    let wallet_client: Arc<dyn crate::services::wallet_client::WalletClient + Send + Sync> =
        Arc::new(MockWalletClient::new());

    let result = cancel_order(
        order_repo,
        inv_client,
        wallet_client,
        order_id,
        user_id,
        &Role::Admin,
        CancelRequest {
            cancellation_reason: "auto-cancel sweep".to_string(),
        },
    )
    .await;

    assert!(result.is_err());
}
