use std::sync::Arc;
use uuid::Uuid;

use crate::infrastructure::worker::process_checkout_request;
use crate::models::checkout_request::CheckoutRequest;
use crate::models::order::CreateOrderRequest;
use crate::models::shipping_address::ShippingAddress;
use crate::repositories::idempotency_repository::MockIdempotencyRepository;
use crate::repositories::order_repository::MockOrderRepository;
use crate::services::inventory_client::MockInventoryClient;
use crate::services::wallet_client::MockWalletClient;

use crate::tests::functional::common::setup_jwt_secret;

fn make_order(order_id: Uuid, titipers_id: Uuid, jastiper_id: Uuid) -> crate::models::order::Order {
    crate::models::order::Order {
        order_id,
        titipers_id,
        jastiper_id,
        product_id: Uuid::new_v4(),
        product_snapshot: serde_json::json!({"product_id": Uuid::new_v4().to_string()}),
        quantity: 1,
        unit_price: 10000,
        service_fee: 1000,
        total_price: 11000,
        status: crate::models::order_state::OrderStatus::Reserving,
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
async fn test_double_saga_execution_skipped_by_idempotency() {
    setup_jwt_secret();
    let order_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let idem_key = Uuid::new_v4();

    let mut idem_repo = MockIdempotencyRepository::new();
    let call_count = std::cell::Cell::new(0u32);
    idem_repo
        .expect_is_processed()
        .times(2)
        .returning(move |_| {
            let n = call_count.get();
            call_count.set(n + 1);
            Ok(n > 0)
        });
    idem_repo
        .expect_mark_processed()
        .times(1)
        .returning(|_, _| Ok(()));

    let mut order_repo = MockOrderRepository::new();
    order_repo
        .expect_find_by_id()
        .times(1)
        .returning(move |_| Ok(Some(make_order(order_id, titipers_id, jastiper_id))));
    order_repo
        .expect_update()
        .returning(move |_, _, _| Ok(make_order(order_id, titipers_id, jastiper_id)));

    let mut inv_client = MockInventoryClient::new();
    inv_client
        .expect_reserve_stock()
        .returning(|_, _, _| Ok(()));

    let mut wallet_client = MockWalletClient::new();
    wallet_client.expect_check_wallet().returning(|_, _| Ok(()));

    let order_repo: Arc<dyn crate::repositories::order_repository::OrderRepository + Send + Sync> =
        Arc::new(order_repo);
    let inv_client: Arc<dyn crate::services::inventory_client::InventoryClient + Send + Sync> =
        Arc::new(inv_client);
    let wallet_client: Arc<dyn crate::services::wallet_client::WalletClient + Send + Sync> =
        Arc::new(wallet_client);
    let idem_repo: Arc<
        dyn crate::repositories::idempotency_repository::IdempotencyRepository + Send + Sync,
    > = Arc::new(idem_repo);

    let base_request = || CheckoutRequest {
        order_id,
        titipers_id,
        jastiper_id,
        req: CreateOrderRequest {
            product_id: Uuid::new_v4(),
            quantity: 1,
            shipping_address: ShippingAddress {
                recipient_name: "Test".to_string(),
                phone_number: "123".to_string(),
                street: "Jln".to_string(),
                kelurahan: "A".to_string(),
                kecamatan: "B".to_string(),
                city: "C".to_string(),
                province: "D".to_string(),
                postal_code: "12345".to_string(),
                notes: None,
            },
            note_to_jastiper: None,
        },
        product: serde_json::json!({}),
        idempotency_key: idem_key,
    };

    let result1: Result<(), crate::error::AppError> = process_checkout_request(
        &order_repo,
        &inv_client,
        &wallet_client,
        &idem_repo,
        base_request(),
    )
    .await;
    assert!(result1.is_ok());

    let result2: Result<(), crate::error::AppError> = process_checkout_request(
        &order_repo,
        &inv_client,
        &wallet_client,
        &idem_repo,
        base_request(),
    )
    .await;
    assert!(result2.is_ok());
}
