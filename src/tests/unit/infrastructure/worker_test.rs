use crate::error::AppError;
use crate::infrastructure::worker::process_checkout_request;
use crate::models::checkout_request::CheckoutRequest;
use crate::models::order::{CreateOrderRequest, Order};
use crate::models::order_state::OrderStatus;
use crate::models::shipping_address::ShippingAddress;
use crate::repositories::idempotency_repository::IdempotencyRepository;
use crate::repositories::idempotency_repository::MockIdempotencyRepository;
use crate::repositories::order_repository::MockOrderRepository;
use crate::repositories::order_repository::OrderRepository;
use crate::services::auth_client::AuthClient;
use crate::services::auth_client::MockAuthClient;
use crate::services::inventory_client::InventoryClient;
use crate::services::inventory_client::MockInventoryClient;
use crate::services::wallet_client::MockWalletClient;
use crate::services::wallet_client::WalletClient;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

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
        expired_at: chrono::Utc::now(),
    }
}

fn make_request(
    order_id: Uuid,
    titipers_id: Uuid,
    jastiper_id: Uuid,
    product_id: Uuid,
) -> CheckoutRequest {
    CheckoutRequest {
        order_id,
        titipers_id,
        jastiper_id,
        req: CreateOrderRequest {
            product_id,
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
        },
        product: json!({
            "name": "Snack",
            "description": "Test",
            "images": ["https://example.test/image.png"],
            "originCountry": "JP",
            "purchaseDate": "2026-01-01",
            "price": 10_000,
            "service_fee": 1_000
        }),
        idempotency_key: order_id,
    }
}

async fn run_process_checkout_request(
    order_repo: MockOrderRepository,
    inventory: MockInventoryClient,
    wallet: MockWalletClient,
    idempotency: MockIdempotencyRepository,
    auth: MockAuthClient,
    request: CheckoutRequest,
) -> Result<(), AppError> {
    let order_repo: Arc<dyn OrderRepository + Send + Sync> = Arc::new(order_repo);
    let inventory: Arc<dyn InventoryClient + Send + Sync> = Arc::new(inventory);
    let wallet: Arc<dyn WalletClient + Send + Sync> = Arc::new(wallet);
    let auth: Arc<dyn AuthClient + Send + Sync> = Arc::new(auth);
    let idempotency: Arc<dyn IdempotencyRepository + Send + Sync> = Arc::new(idempotency);

    process_checkout_request(&order_repo, &inventory, &wallet, &auth, &idempotency, request).await
}

// === Happy Path ===
#[tokio::test]
async fn test_process_checkout_request_success_marks_processed() {
    let order_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let request = make_request(order_id, titipers_id, jastiper_id, product_id);

    let mut order_repo = MockOrderRepository::new();
    let mut inventory = MockInventoryClient::new();
    let mut wallet = MockWalletClient::new();
    let mut idempotency = MockIdempotencyRepository::new();
    let mut auth = MockAuthClient::new();

    idempotency.expect_is_processed().returning(|_| Ok(false));
    order_repo.expect_find_by_id().returning(move |_| {
        Ok(Some(make_order(
            order_id,
            titipers_id,
            jastiper_id,
            OrderStatus::Reserving,
        )))
    });
    wallet.expect_check_wallet().returning(|_, _| Ok(()));
    inventory.expect_reserve_stock().returning(|_, _, _| Ok(()));
    order_repo.expect_update().returning(move |_, status, _| {
        Ok(make_order(
            order_id,
            titipers_id,
            jastiper_id,
            status.clone(),
        ))
    });
    idempotency.expect_mark_processed().returning(|_, _| Ok(()));
    auth.expect_send_order_event().returning(|_, _| Ok(()));

    let result =
        run_process_checkout_request(order_repo, inventory, wallet, idempotency, auth, request).await;

    assert!(result.is_ok());
}

// === Edge Cases ===
#[tokio::test]
async fn test_process_checkout_request_duplicate_message_skips_saga() {
    let request = make_request(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let order_repo = MockOrderRepository::new();
    let inventory = MockInventoryClient::new();
    let wallet = MockWalletClient::new();
    let mut idempotency = MockIdempotencyRepository::new();
    let auth = MockAuthClient::new();
    idempotency.expect_is_processed().returning(|_| Ok(true));

    let result =
        run_process_checkout_request(order_repo, inventory, wallet, idempotency, auth, request).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_process_checkout_request_non_reserving_order_marks_processed() {
    let order_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let request = make_request(order_id, titipers_id, jastiper_id, Uuid::new_v4());
    let mut order_repo = MockOrderRepository::new();
    let inventory = MockInventoryClient::new();
    let wallet = MockWalletClient::new();
    let mut idempotency = MockIdempotencyRepository::new();
    let auth = MockAuthClient::new();

    idempotency.expect_is_processed().returning(|_| Ok(false));
    order_repo.expect_find_by_id().returning(move |_| {
        Ok(Some(make_order(
            order_id,
            titipers_id,
            jastiper_id,
            OrderStatus::Pending,
        )))
    });
    idempotency.expect_mark_processed().returning(|_, _| Ok(()));

    let result =
        run_process_checkout_request(order_repo, inventory, wallet, idempotency, auth, request).await;

    assert!(result.is_ok());
}

// === Error Path ===
#[tokio::test]
async fn test_process_checkout_request_missing_order_returns_not_found() {
    let request = make_request(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let mut order_repo = MockOrderRepository::new();
    let inventory = MockInventoryClient::new();
    let wallet = MockWalletClient::new();
    let mut idempotency = MockIdempotencyRepository::new();
    let auth = MockAuthClient::new();

    idempotency.expect_is_processed().returning(|_| Ok(false));
    order_repo.expect_find_by_id().returning(|_| Ok(None));

    let result =
        run_process_checkout_request(order_repo, inventory, wallet, idempotency, auth, request).await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn test_process_checkout_request_saga_error_returns_error_without_side_effects() {
    let order_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let request = make_request(order_id, titipers_id, jastiper_id, Uuid::new_v4());
    let mut order_repo = MockOrderRepository::new();
    let inventory = MockInventoryClient::new();
    let mut wallet = MockWalletClient::new();
    let mut idempotency = MockIdempotencyRepository::new();
    let auth = MockAuthClient::new();

    idempotency.expect_is_processed().returning(|_| Ok(false));
    order_repo.expect_find_by_id().returning(move |_| {
        Ok(Some(make_order(
            order_id,
            titipers_id,
            jastiper_id,
            OrderStatus::Reserving,
        )))
    });
    wallet
        .expect_check_wallet()
        .returning(|_, _| Err(AppError::UnprocessableEntity("Saldo tidak cukup".to_string())));

    // No expect_update or expect_mark_processed — saga failure should not trigger cancel
    // or idempotency marking in process_checkout_request.

    let result =
        run_process_checkout_request(order_repo, inventory, wallet, idempotency, auth, request).await;

    assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
}

#[tokio::test]
async fn test_process_checkout_request_internal_error_returns_error_without_side_effects() {
    let order_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let request = make_request(order_id, titipers_id, jastiper_id, Uuid::new_v4());
    let mut order_repo = MockOrderRepository::new();
    let inventory = MockInventoryClient::new();
    let mut wallet = MockWalletClient::new();
    let mut idempotency = MockIdempotencyRepository::new();
    let auth = MockAuthClient::new();

    idempotency.expect_is_processed().returning(|_| Ok(false));
    order_repo.expect_find_by_id().returning(move |_| {
        Ok(Some(make_order(
            order_id,
            titipers_id,
            jastiper_id,
            OrderStatus::Reserving,
        )))
    });
    wallet
        .expect_check_wallet()
        .returning(|_, _| Err(AppError::Internal));

    let result =
        run_process_checkout_request(order_repo, inventory, wallet, idempotency, auth, request).await;

    assert!(matches!(result, Err(AppError::Internal)));
}
