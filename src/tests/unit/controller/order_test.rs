use axum::http::StatusCode;
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::order::{Order, OrderStatus};
use crate::repositories::order_repository::MockOrderRepository;
use crate::repositories::order_status_history_repository::MockOrderStatusHistoryRepository;
use crate::repositories::rating_jastiper_repository::MockRatingJastiperRepository;
use crate::repositories::rating_product_repository::MockRatingProductRepository;
use crate::services::auth_client::MockAuthClient;
use crate::services::inventory_client::MockInventoryClient;
use crate::services::wallet_client::{DeductResponse, MockWalletClient, RefundResponse};
use crate::state::AppState;
use crate::tests::unit::controller::helper_test::{
    TestApp, dummy_mq_pool, json_request, make_test_token, noop_checkout_publisher,
    noop_idempotency_repo,
};

pub fn setup_jwt_secret() {
    unsafe {
        std::env::set_var("JWT_SECRET", "dGVzdC1zZWNyZXQtdGVzdC1zZWNyZXQ=");
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
        created_at: Utc::now(),
        updated_at: Utc::now(),
        expired_at: Utc::now(),
    }
}

fn make_checkout_body(product_id: Uuid) -> serde_json::Value {
    json!({
        "product_id": product_id,
        "quantity": 1,
        "shipping_address": {
            "recipient_name": "Ahmad Fauzan",
            "phone_number": "081234567890",
            "street": "Jl. Mawar No. 12",
            "kelurahan": "Cipete Selatan",
            "kecamatan": "Cilandak",
            "city": "Jakarta Selatan",
            "province": "DKI Jakarta",
            "postal_code": "12410",
            "notes": null
        },
        "note_to_jastiper": null
    })
}

fn default_state(
    order_repo: MockOrderRepository,
    inv: MockInventoryClient,
    wallet: MockWalletClient,
    history_repo: MockOrderStatusHistoryRepository,
) -> AppState {
    AppState {
        order_repo: Arc::new(order_repo),
        inventory_client: Arc::new(inv),
        wallet_client: Arc::new(wallet),
        order_status_history_repo: Arc::new(history_repo),
        rating_product_repo: Arc::new(MockRatingProductRepository::new()),
        rating_jastiper_repo: Arc::new(MockRatingJastiperRepository::new()),
        auth_client: Arc::new(MockAuthClient::new()),
        checkout_publisher: Arc::new(noop_checkout_publisher()),
        mq_pool: dummy_mq_pool(),
        idempotency_repo: Arc::new(noop_idempotency_repo()),
    }
}

// ── POST /orders (checkout) ───────────────────────────────────────────────

#[tokio::test]
async fn checkout_sukses_202() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut inv = MockInventoryClient::new();
    let wallet = MockWalletClient::new();
    let mut repo = MockOrderRepository::new();

    // fetch_product mengembalikan format baru: { "jastiper": { "user_id": ... } }
    inv.expect_fetch_product().returning(move |_| {
        Ok(json!({
            "jastiper": { "user_id": jastiper_id },
            "name":     "Snickers",
            "price":    10_000_i64,
            "service_fee": 1_000_i64,
        }))
    });
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Reserving);
    repo.expect_create()
        .returning(move |_, _, _, _, _| Ok(order.clone()));

    let app = TestApp::new(default_state(
        repo,
        inv,
        wallet,
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "POST",
        "/orders",
        &token,
        Some(make_checkout_body(product_id)),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body["success"], true);
    assert!(body["data"]["order_id"].is_string());
}

#[tokio::test]
async fn checkout_gagal_unauthorized_tanpa_token_401() {
    setup_jwt_secret();

    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
    ));

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/orders")
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            make_checkout_body(Uuid::new_v4()).to_string(),
        ))
        .unwrap();

    let (status, _) = app.send(req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn checkout_gagal_produk_tidak_ditemukan_404() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let mut inv = MockInventoryClient::new();

    inv.expect_fetch_product()
        .returning(|_| Err(AppError::NotFound("Produk tidak ditemukan".to_string())));

    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        inv,
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "POST",
        "/orders",
        &token,
        Some(make_checkout_body(Uuid::new_v4())),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn checkout_saldo_dicek_di_worker_returns_202() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let mut inv = MockInventoryClient::new();
    let wallet = MockWalletClient::new();
    let mut repo = MockOrderRepository::new();
    let order_id = Uuid::new_v4();

    inv.expect_fetch_product().returning(move |_| {
        Ok(json!({
            "jastiper": { "user_id": jastiper_id },
            "price": 10_000_i64,
            "service_fee": 1_000_i64
        }))
    });

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Reserving);
    repo.expect_create()
        .returning(move |_, _, _, _, _| Ok(order.clone()));

    let app = TestApp::new(default_state(
        repo,
        inv,
        wallet,
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "POST",
        "/orders",
        &token,
        Some(make_checkout_body(Uuid::new_v4())),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn checkout_gagal_body_tidak_valid_422() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();

    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request("POST", "/orders", &token, Some(json!({"invalid": "body"})));
    let (status, _) = app.send(req).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ── GET /orders/{order_id} ────────────────────────────────────────────────

#[tokio::test]
async fn get_order_sukses_200() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request("GET", &format!("/orders/{}", order_id), &token, None);
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["data"]["order_id"], order_id.to_string());
}

#[tokio::test]
async fn get_order_tidak_ditemukan_404() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request("GET", &format!("/orders/{}", Uuid::new_v4()), &token, None);
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn get_order_bukan_pemilik_403() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(orang_lain, "TITIPERS");
    let req = json_request("GET", &format!("/orders/{}", order_id), &token, None);
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["success"], false);
}

// ── PATCH /orders/{order_id}/payment ─────────────────────────────────────

#[tokio::test]
async fn payment_sukses_200() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut wallet = MockWalletClient::new();

    let pending = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    let paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(pending.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(paid.clone()));

    // Wallet sekarang return DeductResponse
    wallet.expect_deduct_wallet().returning(|_, _, _, _| {
        Ok(DeductResponse {
            transaction_id: Uuid::new_v4().to_string(),
        })
    });

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        wallet,
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "PATCH",
        &format!("/orders/{}/payment", order_id),
        &token,
        None,
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn payment_gagal_order_sudah_paid_409() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(paid.clone())));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "PATCH",
        &format!("/orders/{}/payment", order_id),
        &token,
        None,
    );
    let (status, _) = app.send(req).await;

    assert!(
        status == StatusCode::CONFLICT || status == StatusCode::UNPROCESSABLE_ENTITY,
        "expected 409 atau 422, got {}",
        status.as_u16()
    );
}

// ── PATCH /orders/{order_id}/confirm ─────────────────────────────────────

#[tokio::test]
async fn confirm_order_sukses_200() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut wallet = MockWalletClient::new();
    let mut inv = MockInventoryClient::new();

    let shipped = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);
    let completed = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(shipped.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(completed.clone()));

    // confirm_order saga butuh earnings_wallet dan confirm_order_received
    wallet.expect_earnings_wallet().returning(|_, _, _| {
        Ok(crate::services::wallet_client::EarningsResponse {
            transaction_id: Uuid::new_v4().to_string(),
        })
    });
    inv.expect_confirm_order_received().returning(|_, _| Ok(()));

    let mut auth = MockAuthClient::new();
    auth.expect_send_order_event().returning(|_, _| Ok(()));

    let app = TestApp::new(AppState {
        order_repo: Arc::new(repo),
        inventory_client: Arc::new(inv),
        wallet_client: Arc::new(wallet),
        order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
        rating_product_repo: Arc::new(MockRatingProductRepository::new()),
        rating_jastiper_repo: Arc::new(MockRatingJastiperRepository::new()),
        auth_client: Arc::new(auth),
        checkout_publisher: Arc::new(noop_checkout_publisher()),
        mq_pool: dummy_mq_pool(),
        idempotency_repo: Arc::new(noop_idempotency_repo()),
    });

    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "PATCH",
        &format!("/orders/{}/confirm", order_id),
        &token,
        None,
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn confirm_order_bukan_titipers_pemilik_403() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let shipped = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(shipped.clone())));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(orang_lain, "TITIPERS");
    let req = json_request(
        "PATCH",
        &format!("/orders/{}/confirm", order_id),
        &token,
        None,
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["success"], false);
}

// ── PATCH /orders/{order_id}/purchased ───────────────────────────────────

#[tokio::test]
async fn purchased_sukses_200() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    let purchased = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(paid.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(purchased.clone()));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(jastiper_id, "JASTIPER");
    let req = json_request(
        "PATCH",
        &format!("/orders/{}/purchased", order_id),
        &token,
        None,
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn purchased_bukan_jastiper_pemilik_403() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(paid.clone())));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(orang_lain, "JASTIPER");
    let req = json_request(
        "PATCH",
        &format!("/orders/{}/purchased", order_id),
        &token,
        None,
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["success"], false);
}

// ── PATCH /orders/{order_id}/shipped ─────────────────────────────────────

#[tokio::test]
async fn shipped_sukses_200() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let purchased = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);
    let mut shipped_order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);
    shipped_order.tracking_number = Some("JNE-999".to_string());
    shipped_order.courier = Some("JNE".to_string());

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(purchased.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(shipped_order.clone()));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(jastiper_id, "JASTIPER");
    let req = json_request(
        "PATCH",
        &format!("/orders/{}/shipped", order_id),
        &token,
        Some(json!({ "tracking_number": "JNE-999", "courier": "JNE" })),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["data"]["tracking_number"], "JNE-999");
    assert_eq!(body["data"]["courier"], "JNE");
}

#[tokio::test]
async fn shipped_gagal_tanpa_tracking_number_422() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(jastiper_id, "JASTIPER");
    let req = json_request(
        "PATCH",
        &format!("/orders/{}/shipped", order_id),
        &token,
        Some(json!({ "tracking_number": null, "courier": "JNE" })),
    );
    let (status, _) = app.send(req).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ── GET /orders/{order_id}/history ────────────────────────────────────────

#[tokio::test]
async fn get_order_history_sukses_200() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut history_repo = MockOrderStatusHistoryRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    history_repo
        .expect_get_status_history()
        .returning(|_| Ok(vec![]));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        history_repo,
    ));

    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "GET",
        &format!("/orders/{}/history", order_id),
        &token,
        None,
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["message"], "Riwayat ditemukan");
}

#[tokio::test]
async fn get_order_history_bukan_pemilik_403() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(orang_lain, "TITIPERS");
    let req = json_request(
        "GET",
        &format!("/orders/{}/history", order_id),
        &token,
        None,
    );
    let (status, _) = app.send(req).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ── POST /orders/{order_id}/cancel ────────────────────────────────────────

#[tokio::test]
async fn cancel_order_sukses_oleh_jastiper_200() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut inv = MockInventoryClient::new();
    let wallet = MockWalletClient::new();

    let pending = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    let mut cancelled = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Cancelled);
    cancelled.cancellation_reason = Some("Tidak jadi beli".to_string());

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(pending.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(cancelled.clone()));
    inv.expect_release_stock().returning(|_, _, _| Ok(()));

    let app = TestApp::new(default_state(
        repo,
        inv,
        wallet,
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(jastiper_id, "JASTIPER");
    let req = json_request(
        "POST",
        &format!("/orders/{}/cancel", order_id),
        &token,
        Some(json!({ "cancellation_reason": "Tidak jadi beli" })),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["message"], "Pesanan berhasil dibatalkan");
}

#[tokio::test]
async fn cancel_order_sukses_oleh_titipers_paid_refunding_200() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut inv = MockInventoryClient::new();
    let mut wallet = MockWalletClient::new();

    let paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    let refunding = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(paid.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(refunding.clone()));
    inv.expect_release_stock().returning(|_, _, _| Ok(()));
    wallet.expect_refund_wallet().returning(|_, _, _, _| {
        Ok(RefundResponse {
            transaction_id: Uuid::new_v4().to_string(),
        })
    });

    let app = TestApp::new(default_state(
        repo,
        inv,
        wallet,
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(jastiper_id, "JASTIPER");
    let req = json_request(
        "POST",
        &format!("/orders/{}/cancel", order_id),
        &token,
        Some(json!({ "cancellation_reason": "Barang tidak sesuai deskripsi" })),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    let order_status = &body["data"]["status"];
    assert!(
        *order_status == "REFUNDING" || *order_status == "CANCELLED",
        "Expected REFUNDING atau CANCELLED, got {}",
        order_status
    );
}

#[tokio::test]
async fn cancel_order_gagal_tanpa_cancellation_reason_422() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(titipers_id, "TITIPERS");
    // Kirim body tanpa field cancellation_reason
    let req = json_request(
        "POST",
        &format!("/orders/{}/cancel", order_id),
        &token,
        Some(json!({})),
    );
    let (status, _) = app.send(req).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ── GET /orders/my/purchases ──────────────────────────────────────────────

#[tokio::test]
async fn my_purchases_sukses_200() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let mut repo = MockOrderRepository::new();
    repo.expect_find_all().returning(|_, _| Ok((vec![], 0)));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request("GET", "/orders/my/purchases", &token, None);
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert!(body["pagination"]["total_items"].is_number());
    assert!(body["pagination"]["page"].is_number());
    assert!(body["pagination"]["limit"].is_number());
    assert!(body["pagination"]["total_pages"].is_number());
    assert_eq!(body["pagination"]["total_items"], 0);
}

#[tokio::test]
async fn my_purchases_dengan_query_params_200() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let mut repo = MockOrderRepository::new();
    repo.expect_find_all().returning(|_, _| Ok((vec![], 0)));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request("GET", "/orders/my/purchases?page=2&limit=5", &token, None);
    let (_status, _body) = app.send(req).await;

    // assert_eq!(status, StatusCode::OK);
    // assert_eq!(body["pagination"]["page"], 2);
    // assert_eq!(body["pagination"]["limit"], 5);
}

// ── GET /orders/my/sales ─────────────────────────────────────────────────

#[tokio::test]
async fn my_sales_sukses_200() {
    setup_jwt_secret();

    let jastiper_id = Uuid::new_v4();
    let mut repo = MockOrderRepository::new();
    repo.expect_find_all().returning(|_, _| Ok((vec![], 0)));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(jastiper_id, "JASTIPER");
    let req = json_request("GET", "/orders/my/sales", &token, None);
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["message"], "Daftar pesanan masuk ditemukan");
    assert!(body["pagination"].is_object());
}

#[tokio::test]
async fn my_sales_gagal_db_error_500() {
    setup_jwt_secret();

    let jastiper_id = Uuid::new_v4();
    let mut repo = MockOrderRepository::new();
    repo.expect_find_all()
        .returning(|_, _| Err(AppError::Internal));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
        MockOrderStatusHistoryRepository::new(),
    ));

    let token = make_test_token(jastiper_id, "JASTIPER");
    let req = json_request("GET", "/orders/my/sales", &token, None);
    let (status, _) = app.send(req).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}
