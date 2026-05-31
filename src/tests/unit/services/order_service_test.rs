use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use crate::error::AppError;
use crate::infrastructure::publisher::MockCheckoutPublisher;
use crate::models::filter_pagination::{OrderQueryParams, PaginationParams};
use crate::models::order::{
    CancelRequest, CreateOrderRequest, Order, ShippedRequest, UpdateStatusRequest,
};
use crate::models::order_state::OrderStatus;
use crate::models::role::Role;
use crate::models::shipping_address::ShippingAddress;
use crate::repositories::idempotency_repository::MockIdempotencyRepository;
use crate::repositories::order_repository::MockOrderRepository;
use crate::repositories::order_status_history_repository::MockOrderStatusHistoryRepository;
use crate::services::auth_client::MockAuthClient;
use crate::services::inventory_client::MockInventoryClient;
use crate::services::order;
use crate::services::wallet_client::{
    DeductResponse, EarningsResponse, MockWalletClient, RefundResponse,
};

// ──────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────

fn make_order(order_id: Uuid, titipers_id: Uuid, jastiper_id: Uuid, status: OrderStatus) -> Order {
    Order {
        order_id,
        titipers_id,
        jastiper_id,
        product_id: Uuid::new_v4(),
        product_snapshot: json!({ "product_id": Uuid::new_v4().to_string() }),
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

fn make_create_request(product_id: Uuid) -> CreateOrderRequest {
    CreateOrderRequest {
        product_id,
        quantity: 1,
        shipping_address: ShippingAddress {
            recipient_name: "Ahmad Fauzan".to_string(),
            phone_number: "081234567890".to_string(),
            street: "Jl. Mawar No. 12".to_string(),
            kelurahan: "Cipete Selatan".to_string(),
            kecamatan: "Cilandak".to_string(),
            city: "Kota Jakarta Selatan".to_string(),
            province: "DKI Jakarta".to_string(),
            postal_code: "12410".to_string(),
            notes: None,
        },
        note_to_jastiper: None,
        idempotency_key: None,
    }
}

fn make_order_query_params() -> OrderQueryParams {
    OrderQueryParams {
        pagination: PaginationParams {
            page: Some(1),
            limit: Some(10),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn make_checkout_publisher_ok() -> MockCheckoutPublisher {
    let mut publisher = MockCheckoutPublisher::new();
    publisher.expect_publish().returning(|_| Ok(()));
    publisher
}

// ──────────────────────────────────────────────────────────────
// checkout
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn checkout_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();

    let mut inv = MockInventoryClient::new();
    let publisher = make_checkout_publisher_ok();
    let mut repo = MockOrderRepository::new();

    let product_json = json!({
        "jastiper": { "user_id": jastiper_id },
        "name": "Snickers",
        "description": "Coklat",
        "images": ["http://img.url"],
        "originCountry": "Japan",
        "purchaseDate": "2026-01-01",
        "price": 10_000_i64,
        "service_fee": 1_000_i64,
    });

    inv.expect_fetch_product()
        .returning(move |_| Ok(product_json.clone()));

    let expected_order = make_order(
        Uuid::new_v4(),
        titipers_id,
        jastiper_id,
        OrderStatus::Pending,
    );
    repo.expect_create()
        .returning(move |_, _, _, _, _, _| Ok(expected_order.clone()));

    let req = make_create_request(product_id);
    let result = order::checkout(
        Arc::new(repo),
        Arc::new(inv),
        Arc::new(publisher),
        Arc::new(MockIdempotencyRepository::new()),
        titipers_id,
        req,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn checkout_gagal_jastiper_beli_produk_sendiri() {
    let user_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();

    let mut inv = MockInventoryClient::new();
    let publisher = MockCheckoutPublisher::new();
    let repo = MockOrderRepository::new();

    let product_json = json!({
        "jastiper": { "user_id": user_id },
        "price": 10_000_i64,
        "service_fee": 1_000_i64,
    });

    inv.expect_fetch_product()
        .returning(move |_| Ok(product_json.clone()));

    let req = make_create_request(product_id);
    let result = order::checkout(
        Arc::new(repo),
        Arc::new(inv),
        Arc::new(publisher),
        Arc::new(MockIdempotencyRepository::new()),
        user_id,
        req,
    )
    .await;

    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn checkout_gagal_fetch_product_error() {
    let titipers_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();

    let mut inv = MockInventoryClient::new();
    let publisher = MockCheckoutPublisher::new();
    let repo = MockOrderRepository::new();

    inv.expect_fetch_product()
        .returning(|_| Err(AppError::Internal));

    let req = make_create_request(product_id);
    let result = order::checkout(
        Arc::new(repo),
        Arc::new(inv),
        Arc::new(publisher),
        Arc::new(MockIdempotencyRepository::new()),
        titipers_id,
        req,
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn checkout_gagal_jastiper_id_tidak_valid_di_product() {
    let titipers_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();

    let mut inv = MockInventoryClient::new();
    let publisher = MockCheckoutPublisher::new();
    let repo = MockOrderRepository::new();

    // jastiper.user_id bukan UUID valid → parse error → AppError::Internal
    let product_json = json!({
        "jastiper": { "user_id": "bukan-uuid" },
        "price": 10_000_i64,
        "service_fee": 1_000_i64,
    });

    inv.expect_fetch_product()
        .returning(move |_| Ok(product_json.clone()));

    let req = make_create_request(product_id);
    let result = order::checkout(
        Arc::new(repo),
        Arc::new(inv),
        Arc::new(publisher),
        Arc::new(MockIdempotencyRepository::new()),
        titipers_id,
        req,
    )
    .await;

    assert!(matches!(result, Err(AppError::Internal)));
}

#[tokio::test]
async fn checkout_gagal_publish_error() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();

    let mut inv = MockInventoryClient::new();
    let mut publisher = MockCheckoutPublisher::new();
    let mut repo = MockOrderRepository::new();

    let product_json = json!({
        "jastiper": { "user_id": jastiper_id },
        "price": 10_000_i64,
        "service_fee": 1_000_i64,
    });

    inv.expect_fetch_product()
        .returning(move |_| Ok(product_json.clone()));
    let expected_order = make_order(
        Uuid::new_v4(),
        titipers_id,
        jastiper_id,
        OrderStatus::Reserving,
    );
    repo.expect_create()
        .returning(move |_, _, _, _, _, _| Ok(expected_order.clone()));
    publisher
        .expect_publish()
        .returning(|_| Err(AppError::Internal));

    let req = make_create_request(product_id);
    let result = order::checkout(
        Arc::new(repo),
        Arc::new(inv),
        Arc::new(publisher),
        Arc::new(MockIdempotencyRepository::new()),
        titipers_id,
        req,
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn checkout_gagal_create_order_error() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();

    let mut inv = MockInventoryClient::new();
    let publisher = make_checkout_publisher_ok();
    let mut repo = MockOrderRepository::new();

    let product_json = json!({
        "jastiper": { "user_id": jastiper_id },
        "price": 10_000_i64,
        "service_fee": 1_000_i64,
    });

    inv.expect_fetch_product()
        .returning(move |_| Ok(product_json.clone()));
    repo.expect_create()
        .returning(|_, _, _, _, _, _| Err(AppError::Internal));

    let req = make_create_request(product_id);
    let result = order::checkout(
        Arc::new(repo),
        Arc::new(inv),
        Arc::new(publisher),
        Arc::new(MockIdempotencyRepository::new()),
        titipers_id,
        req,
    )
    .await;

    assert!(result.is_err());
}

// ──────────────────────────────────────────────────────────────
// get_order
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_order_sukses_sebagai_titipers() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let expected = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(expected.clone())));

    let result = order::get_order(Arc::new(repo), order_id, titipers_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn get_order_sukses_sebagai_jastiper() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let expected = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(expected.clone())));

    let result = order::get_order(Arc::new(repo), order_id, jastiper_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn get_order_gagal_tidak_ditemukan() {
    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

    let result = order::get_order(Arc::new(repo), Uuid::new_v4(), Uuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn get_order_gagal_bukan_pemilik() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let expected = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(expected.clone())));

    let result = order::get_order(Arc::new(repo), order_id, orang_lain).await;
    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn get_order_gagal_db_error() {
    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(|_| Err(AppError::Internal));

    let result = order::get_order(Arc::new(repo), Uuid::new_v4(), Uuid::new_v4()).await;
    assert!(result.is_err());
}

// ──────────────────────────────────────────────────────────────
// update_status
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_status_sukses_jastiper_ke_purchased() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    let updated = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(updated.clone()));

    let req = UpdateStatusRequest {
        status: OrderStatus::Purchased,
        notes: None,
        tracking_number: None,
        courier: None,
        cancellation_reason: None,
    };

    let result =
        order::update_status(Arc::new(repo), order_id, jastiper_id, &Role::Jastiper, req).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, OrderStatus::Purchased);
}

#[tokio::test]
async fn update_status_gagal_order_tidak_ditemukan() {
    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

    let req = UpdateStatusRequest {
        status: OrderStatus::Purchased,
        notes: None,
        tracking_number: None,
        courier: None,
        cancellation_reason: None,
    };

    let result = order::update_status(
        Arc::new(repo),
        Uuid::new_v4(),
        Uuid::new_v4(),
        &Role::Jastiper,
        req,
    )
    .await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn update_status_gagal_jastiper_bukan_pemilik() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let jastiper_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = UpdateStatusRequest {
        status: OrderStatus::Purchased,
        notes: None,
        tracking_number: None,
        courier: None,
        cancellation_reason: None,
    };

    let result = order::update_status(
        Arc::new(repo),
        order_id,
        jastiper_lain,
        &Role::Jastiper,
        req,
    )
    .await;
    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn update_status_gagal_shipped_tanpa_tracking_number() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = UpdateStatusRequest {
        status: OrderStatus::Shipped,
        notes: None,
        tracking_number: None,
        courier: Some("JNE".to_string()),
        cancellation_reason: None,
    };

    let result =
        order::update_status(Arc::new(repo), order_id, jastiper_id, &Role::Jastiper, req).await;
    assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
}

#[tokio::test]
async fn update_status_gagal_shipped_tanpa_courier() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = UpdateStatusRequest {
        status: OrderStatus::Shipped,
        notes: None,
        tracking_number: Some("JNE-123".to_string()),
        courier: None,
        cancellation_reason: None,
    };

    let result =
        order::update_status(Arc::new(repo), order_id, jastiper_id, &Role::Jastiper, req).await;
    assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
}

#[tokio::test]
async fn update_status_gagal_titipers_bukan_pemilik() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let titipers_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = UpdateStatusRequest {
        status: OrderStatus::Completed,
        notes: None,
        tracking_number: None,
        courier: None,
        cancellation_reason: None,
    };

    let result = order::update_status(
        Arc::new(repo),
        order_id,
        titipers_lain,
        &Role::Titipers,
        req,
    )
    .await;
    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

// ──────────────────────────────────────────────────────────────
// cancel_status
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn cancel_status_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    let updated = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Cancelled);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(updated.clone()));

    let req = UpdateStatusRequest {
        status: OrderStatus::Cancelled,
        notes: Some("Dibatalkan".to_string()),
        tracking_number: None,
        courier: None,
        cancellation_reason: Some("Tidak jadi beli".to_string()),
    };

    let result =
        order::cancel_status(Arc::new(repo), order_id, jastiper_id, &Role::Jastiper, req).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn cancel_status_gagal_order_tidak_ditemukan() {
    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

    let req = UpdateStatusRequest {
        status: OrderStatus::Cancelled,
        notes: None,
        tracking_number: None,
        courier: None,
        cancellation_reason: None,
    };

    let result = order::cancel_status(
        Arc::new(repo),
        Uuid::new_v4(),
        Uuid::new_v4(),
        &Role::Titipers,
        req,
    )
    .await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

// ──────────────────────────────────────────────────────────────
// payment
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn payment_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut wallet = MockWalletClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    let paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(paid.clone()));

    wallet.expect_deduct_wallet().returning(|_, _, _, _| {
        Ok(DeductResponse {
            transaction_id: "txn-123".to_string(),
        })
    });

    let result = order::payment(Arc::new(repo), Arc::new(wallet), titipers_id, order_id).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, OrderStatus::Paid);
}

#[tokio::test]
async fn payment_gagal_bukan_pemilik() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let wallet = MockWalletClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let result = order::payment(Arc::new(repo), Arc::new(wallet), orang_lain, order_id).await;
    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn payment_gagal_status_bukan_pending() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let wallet = MockWalletClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let result = order::payment(Arc::new(repo), Arc::new(wallet), titipers_id, order_id).await;
    assert!(matches!(result, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn payment_gagal_deduct_wallet_error() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut wallet = MockWalletClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    wallet
        .expect_deduct_wallet()
        .returning(|_, _, _, _| Err(AppError::Internal));

    let result = order::payment(Arc::new(repo), Arc::new(wallet), titipers_id, order_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn payment_gagal_order_tidak_ditemukan() {
    let mut repo = MockOrderRepository::new();
    let wallet = MockWalletClient::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

    let result = order::payment(
        Arc::new(repo),
        Arc::new(wallet),
        Uuid::new_v4(),
        Uuid::new_v4(),
    )
    .await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

// ──────────────────────────────────────────────────────────────
// confirm_order
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn confirm_order_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut wallet = MockWalletClient::new();
    let mut inv = MockInventoryClient::new();
    let mut auth = MockAuthClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);
    let completed = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(completed.clone()));

    wallet.expect_earnings_wallet().returning(|_, _, _| {
        Ok(EarningsResponse {
            transaction_id: "txn-earn".to_string(),
        })
    });
    inv.expect_confirm_order_received().returning(|_, _| Ok(()));
    auth.expect_send_order_event().returning(|_, _| Ok(()));

    let result = order::confirm_order(
        Arc::new(repo),
        Arc::new(wallet),
        Arc::new(inv),
        Arc::new(auth),
        titipers_id,
        order_id,
    )
    .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn confirm_order_gagal_bukan_titipers_pemilik() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let titipers_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let wallet = MockWalletClient::new();
    let inv = MockInventoryClient::new();
    let auth = MockAuthClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let result = order::confirm_order(
        Arc::new(repo),
        Arc::new(wallet),
        Arc::new(inv),
        Arc::new(auth),
        titipers_lain,
        order_id,
    )
    .await;
    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn confirm_order_gagal_status_bukan_shipped() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let wallet = MockWalletClient::new();
    let inv = MockInventoryClient::new();
    let auth = MockAuthClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let result = order::confirm_order(
        Arc::new(repo),
        Arc::new(wallet),
        Arc::new(inv),
        Arc::new(auth),
        titipers_id,
        order_id,
    )
    .await;
    assert!(matches!(result, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn confirm_order_gagal_order_tidak_ditemukan() {
    let mut repo = MockOrderRepository::new();
    let wallet = MockWalletClient::new();
    let inv = MockInventoryClient::new();
    let auth = MockAuthClient::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

    let result = order::confirm_order(
        Arc::new(repo),
        Arc::new(wallet),
        Arc::new(inv),
        Arc::new(auth),
        Uuid::new_v4(),
        Uuid::new_v4(),
    )
    .await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn confirm_order_gagal_send_order_event_warn() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut wallet = MockWalletClient::new();
    let mut inv = MockInventoryClient::new();
    let mut auth = MockAuthClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);
    let completed = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(completed.clone()));

    wallet.expect_earnings_wallet().returning(|_, _, _| {
        Ok(EarningsResponse {
            transaction_id: "txn-earn".to_string(),
        })
    });
    inv.expect_confirm_order_received().returning(|_, _| Ok(()));
    auth.expect_send_order_event()
        .returning(|_, _| Err(AppError::Internal));

    let result = order::confirm_order(
        Arc::new(repo),
        Arc::new(wallet),
        Arc::new(inv),
        Arc::new(auth),
        titipers_id,
        order_id,
    )
    .await;
    assert!(result.is_ok());
}

// ──────────────────────────────────────────────────────────────
// purchased
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn purchased_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    let updated = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(updated.clone()));

    let result = order::purchased(Arc::new(repo), order_id, jastiper_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn purchased_gagal_bukan_jastiper_pemilik() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let jastiper_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let result = order::purchased(Arc::new(repo), order_id, jastiper_lain).await;
    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

// ──────────────────────────────────────────────────────────────
// shipped
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn shipped_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);
    let updated = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(updated.clone()));

    let req = ShippedRequest {
        tracking_number: Some("JNE-999".to_string()),
        courier: Some("JNE".to_string()),
    };

    let result = order::shipped(Arc::new(repo), order_id, jastiper_id, req).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn shipped_gagal_tanpa_tracking_number() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = ShippedRequest {
        tracking_number: None,
        courier: Some("JNE".to_string()),
    };

    let result = order::shipped(Arc::new(repo), order_id, jastiper_id, req).await;
    assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
}

// ──────────────────────────────────────────────────────────────
// get_order_history
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_order_history_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut history_repo = MockOrderStatusHistoryRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    history_repo
        .expect_get_status_history()
        .returning(|_| Ok(vec![]));

    let result = order::get_order_history(
        Arc::new(repo),
        Arc::new(history_repo),
        order_id,
        titipers_id,
    )
    .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn get_order_history_gagal_order_tidak_ditemukan() {
    let mut repo = MockOrderRepository::new();
    let history_repo = MockOrderStatusHistoryRepository::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

    let result = order::get_order_history(
        Arc::new(repo),
        Arc::new(history_repo),
        Uuid::new_v4(),
        Uuid::new_v4(),
    )
    .await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn get_order_history_gagal_bukan_pemilik() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let history_repo = MockOrderStatusHistoryRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let result =
        order::get_order_history(Arc::new(repo), Arc::new(history_repo), order_id, orang_lain)
            .await;
    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

// ──────────────────────────────────────────────────────────────
// cancel_order
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn cancel_order_sukses_paid_oleh_jastiper() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut inv = MockInventoryClient::new();
    let mut wallet = MockWalletClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    let refunding = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);
    let order_clone = order.clone();

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order_clone.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(refunding.clone()));

    inv.expect_release_stock().returning(|_, _, _| Ok(()));
    wallet.expect_refund_wallet().returning(|_, _, _, _| {
        Ok(RefundResponse {
            transaction_id: "".to_string(),
        })
    });

    let req = CancelRequest {
        cancellation_reason: "Tidak jadi beli".to_string(),
    };

    let result = order::cancel_order(
        Arc::new(repo),
        Arc::new(inv),
        Arc::new(wallet),
        order_id,
        jastiper_id,
        &Role::Jastiper,
        req,
    )
    .await;

    assert!(result.is_ok(), "cancel_order gagal: {:?}", result);
}

#[tokio::test]
async fn cancel_order_gagal_order_tidak_ditemukan() {
    let mut repo = MockOrderRepository::new();
    let inv = MockInventoryClient::new();
    let wallet = MockWalletClient::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

    let req = CancelRequest {
        cancellation_reason: "Test".to_string(),
    };

    let result = order::cancel_order(
        Arc::new(repo),
        Arc::new(inv),
        Arc::new(wallet),
        Uuid::new_v4(),
        Uuid::new_v4(),
        &Role::Titipers,
        req,
    )
    .await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn cancel_order_gagal_titipers_bukan_pemilik() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let titipers_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let inv = MockInventoryClient::new();
    let wallet = MockWalletClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = CancelRequest {
        cancellation_reason: "Test".to_string(),
    };

    let result = order::cancel_order(
        Arc::new(repo),
        Arc::new(inv),
        Arc::new(wallet),
        order_id,
        titipers_lain,
        &Role::Titipers,
        req,
    )
    .await;

    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn cancel_order_gagal_jastiper_bukan_pemilik() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let jastiper_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let inv = MockInventoryClient::new();
    let wallet = MockWalletClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = CancelRequest {
        cancellation_reason: "Test".to_string(),
    };

    let result = order::cancel_order(
        Arc::new(repo),
        Arc::new(inv),
        Arc::new(wallet),
        order_id,
        jastiper_lain,
        &Role::Jastiper,
        req,
    )
    .await;

    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

// ──────────────────────────────────────────────────────────────
// my_purchases & my_sales
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn my_purchases_sukses() {
    let titipers_id = Uuid::new_v4();
    let mut repo = MockOrderRepository::new();
    repo.expect_find_all().returning(|_, _| Ok((vec![], 0)));

    let result = order::my_purchases(Arc::new(repo), titipers_id, make_order_query_params()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().1, 0);
}

#[tokio::test]
async fn my_purchases_gagal_db_error() {
    let mut repo = MockOrderRepository::new();
    repo.expect_find_all()
        .returning(|_, _| Err(AppError::Internal));

    let result =
        order::my_purchases(Arc::new(repo), Uuid::new_v4(), make_order_query_params()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn my_sales_sukses() {
    let jastiper_id = Uuid::new_v4();
    let mut repo = MockOrderRepository::new();
    repo.expect_find_all().returning(|_, _| Ok((vec![], 0)));

    let result = order::my_sales(Arc::new(repo), jastiper_id, make_order_query_params()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn my_sales_gagal_db_error() {
    let mut repo = MockOrderRepository::new();
    repo.expect_find_all()
        .returning(|_, _| Err(AppError::Internal));

    let result = order::my_sales(Arc::new(repo), Uuid::new_v4(), make_order_query_params()).await;
    assert!(result.is_err());
}

// === Error Path ===

#[tokio::test]
async fn test_update_status_find_by_id_db_error_returns_error() {
    // Arrange
    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(|_| Err(AppError::Internal));
    let req = UpdateStatusRequest {
        status: OrderStatus::Purchased,
        notes: None,
        tracking_number: None,
        courier: None,
        cancellation_reason: None,
    };

    // Act
    let result = order::update_status(
        Arc::new(repo),
        Uuid::new_v4(),
        Uuid::new_v4(),
        &Role::Jastiper,
        req,
    )
    .await;

    // Assert
    assert!(matches!(result, Err(AppError::Internal)));
}

#[tokio::test]
async fn test_update_status_update_db_error_returns_error() {
    // Arrange
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(|_, _, _| Err(AppError::Internal));
    let req = UpdateStatusRequest {
        status: OrderStatus::Purchased,
        notes: None,
        tracking_number: None,
        courier: None,
        cancellation_reason: None,
    };

    // Act
    let result =
        order::update_status(Arc::new(repo), order_id, jastiper_id, &Role::Jastiper, req).await;

    // Assert
    assert!(matches!(result, Err(AppError::Internal)));
}

#[tokio::test]
async fn test_cancel_status_find_by_id_db_error_returns_error() {
    // Arrange
    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(|_| Err(AppError::Internal));
    let req = UpdateStatusRequest {
        status: OrderStatus::Cancelled,
        notes: None,
        tracking_number: None,
        courier: None,
        cancellation_reason: Some("cancel".to_string()),
    };

    // Act
    let result = order::cancel_status(
        Arc::new(repo),
        Uuid::new_v4(),
        Uuid::new_v4(),
        &Role::Admin,
        req,
    )
    .await;

    // Assert
    assert!(matches!(result, Err(AppError::Internal)));
}

#[tokio::test]
async fn test_cancel_status_update_db_error_returns_error() {
    // Arrange
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(|_, _, _| Err(AppError::Internal));
    let req = UpdateStatusRequest {
        status: OrderStatus::Cancelled,
        notes: None,
        tracking_number: None,
        courier: None,
        cancellation_reason: Some("cancel".to_string()),
    };

    // Act
    let result =
        order::cancel_status(Arc::new(repo), order_id, jastiper_id, &Role::Admin, req).await;

    // Assert
    assert!(matches!(result, Err(AppError::Internal)));
}

#[tokio::test]
async fn test_confirm_order_find_by_id_db_error_returns_error() {
    // Arrange
    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(|_| Err(AppError::Internal));

    // Act
    let result = order::confirm_order(
        Arc::new(repo),
        Arc::new(MockWalletClient::new()),
        Arc::new(MockInventoryClient::new()),
        Arc::new(MockAuthClient::new()),
        Uuid::new_v4(),
        Uuid::new_v4(),
    )
    .await;

    // Assert
    assert!(matches!(result, Err(AppError::Internal)));
}

#[tokio::test]
async fn test_get_order_history_history_repo_error_returns_error() {
    // Arrange
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let mut repo = MockOrderRepository::new();
    let mut history_repo = MockOrderStatusHistoryRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    history_repo
        .expect_get_status_history()
        .returning(|_| Err(AppError::Internal));

    // Act
    let result = order::get_order_history(
        Arc::new(repo),
        Arc::new(history_repo),
        order_id,
        titipers_id,
    )
    .await;

    // Assert
    assert!(matches!(result, Err(AppError::Internal)));
}
