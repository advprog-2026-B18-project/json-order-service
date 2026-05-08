use axum::http::StatusCode;
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use std::sync::Once;
use uuid::Uuid;

static INIT: Once = Once::new();

use crate::error::AppError;
use crate::models::order::{Order, OrderStatus};
use crate::repositories::order_repository::MockOrderRepository;
use crate::repositories::order_status_history_repository::MockOrderStatusHistoryRepository;
use crate::repositories::rating_jastiper_repository::MockRatingJastiperRepository;
use crate::repositories::rating_product_repository::MockRatingProductRepository;
use crate::services::auth_client::MockAuthClient;
use crate::services::inventory_client::MockInventoryClient;
use crate::services::wallet_client::{MockWalletClient, RefundResponse};
use crate::state::AppState;
use crate::tests::unit::controller::helper_test::{TestApp, json_request, make_test_token};

// ──────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────

pub fn setup_jwt_secret() {
    INIT.call_once(|| unsafe {
        std::env::set_var("JWT_SECRET", "dGVzdC1zZWNyZXQtdGVzdC1zZWNyZXQ=");
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-internal-key");
    });
}

fn make_order(order_id: Uuid, titipers_id: Uuid, jastiper_id: Uuid, status: OrderStatus) -> Order {
    Order {
        order_id,
        titipers_id,
        jastiper_id,
        product_id: Uuid::new_v4(),
        product_snapshot: json!({ "product_id": Uuid::new_v4().to_string() }),
        quantity: 2,
        unit_price: 50_000,
        service_fee: 5_000,
        total_price: 110_000,
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
    }
}

fn default_state(
    order_repo: MockOrderRepository,
    inv: MockInventoryClient,
    wallet: MockWalletClient,
) -> AppState {
    AppState {
        order_repo: Arc::new(order_repo),
        inventory_client: Arc::new(inv),
        wallet_client: Arc::new(wallet),
        order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
        rating_product_repo: Arc::new(MockRatingProductRepository::new()),
        rating_jastiper_repo: Arc::new(MockRatingJastiperRepository::new()),
        auth_client: Arc::new(MockAuthClient::new()),
    }
}

fn cancel_body() -> serde_json::Value {
    json!({ "cancellation_reason": "Admin force cancel karena pelanggaran" })
}

// ──────────────────────────────────────────────────────────────
// GET /admin/orders
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn admin_get_all_sukses_200() {
    setup_jwt_secret();

    let admin_id = Uuid::new_v4();
    let mut repo = MockOrderRepository::new();

    repo.expect_find_all().returning(|_, _| Ok((vec![], 0)));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
    ));
    let token = make_test_token(admin_id, "ADMIN");
    let req = json_request("GET", "/admin/orders", &token, None);
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert!(body["data"].is_array());
    assert_eq!(body["pagination"]["total_items"], 0);
}

#[tokio::test]
async fn admin_get_all_sukses_dengan_data_200() {
    setup_jwt_secret();

    let admin_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    repo.expect_find_all()
        .returning(move |_, _| Ok((vec![order.clone()], 1)));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
    ));
    let token = make_test_token(admin_id, "ADMIN");
    let req = json_request("GET", "/admin/orders", &token, None);
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["pagination"]["total_items"], 1);
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn admin_get_all_gagal_unauthorized_401() {
    setup_jwt_secret();

    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockInventoryClient::new(),
        MockWalletClient::new(),
    ));

    let req = axum::http::Request::builder()
        .method("GET")
        .uri("/admin/orders")
        .body(axum::body::Body::empty())
        .unwrap();

    let (status, _) = app.send(req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_get_all_gagal_bukan_admin_403() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockInventoryClient::new(),
        MockWalletClient::new(),
    ));
    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request("GET", "/admin/orders", &token, None);
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn admin_get_all_gagal_jastiper_bukan_admin_403() {
    setup_jwt_secret();

    let jastiper_id = Uuid::new_v4();
    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockInventoryClient::new(),
        MockWalletClient::new(),
    ));
    let token = make_test_token(jastiper_id, "JASTIPER");
    let req = json_request("GET", "/admin/orders", &token, None);
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["success"], false);
}

// ──────────────────────────────────────────────────────────────
// GET /admin/orders/{order_id}
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn admin_get_order_sukses_200() {
    setup_jwt_secret();

    let admin_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
    ));
    let token = make_test_token(admin_id, "ADMIN");
    let req = json_request("GET", &format!("/admin/orders/{}", order_id), &token, None);
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["message"], "OK");
    assert_eq!(body["data"]["order_id"], order_id.to_string());
}

#[tokio::test]
async fn admin_get_order_gagal_unauthorized_401() {
    setup_jwt_secret();

    let order_id = Uuid::new_v4();
    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockInventoryClient::new(),
        MockWalletClient::new(),
    ));

    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/admin/orders/{}", order_id))
        .body(axum::body::Body::empty())
        .unwrap();

    let (status, _) = app.send(req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_get_order_gagal_bukan_admin_403() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockInventoryClient::new(),
        MockWalletClient::new(),
    ));
    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request("GET", &format!("/admin/orders/{}", order_id), &token, None);
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn admin_get_order_gagal_order_tidak_ditemukan_404() {
    setup_jwt_secret();

    let admin_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
    ));
    let token = make_test_token(admin_id, "ADMIN");
    let req = json_request("GET", &format!("/admin/orders/{}", order_id), &token, None);
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["success"], false);
}

// ──────────────────────────────────────────────────────────────
// POST /admin/orders/{order_id}/force-cancel
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn admin_force_cancel_sukses_200() {
    setup_jwt_secret();

    let admin_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut inv = MockInventoryClient::new();
    let mut wallet = MockWalletClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    let cancelled = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);

    let order_clone = order.clone();
    let cancelled_clone = cancelled.clone();
    let mut call_count = 0;
    repo.expect_find_by_id().returning(move |_| {
        call_count += 1;
        if call_count == 1 {
            Ok(Some(order_clone.clone()))
        } else {
            Ok(Some(cancelled_clone.clone()))
        }
    });
    repo.expect_update()
        .returning(move |_, _, _| Ok(cancelled.clone()));

    // force_cancel memanggil release_stock dan refund_wallet secara best-effort (error diabaikan)
    inv.expect_release_stock().returning(|_, _, _| Ok(()));
    wallet.expect_refund_wallet().returning(|_, _, _, _| {
        Ok(RefundResponse {
            transaction_id: "".to_string(),
        })
    });

    let app = TestApp::new(default_state(repo, inv, wallet));
    let token = make_test_token(admin_id, "ADMIN");
    let req = json_request(
        "POST",
        &format!("/admin/orders/{}/force-cancel", order_id),
        &token,
        Some(cancel_body()),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["message"], "Pesanan berhasil dibatalkan");
    assert!(body["data"]["order_id"].is_string());
}

#[tokio::test]
async fn admin_force_cancel_gagal_unauthorized_401() {
    setup_jwt_secret();

    let order_id = Uuid::new_v4();
    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockInventoryClient::new(),
        MockWalletClient::new(),
    ));

    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/admin/orders/{}/force-cancel", order_id))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(cancel_body().to_string()))
        .unwrap();

    let (status, _) = app.send(req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_force_cancel_gagal_bukan_admin_403() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockInventoryClient::new(),
        MockWalletClient::new(),
    ));
    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "POST",
        &format!("/admin/orders/{}/force-cancel", order_id),
        &token,
        Some(cancel_body()),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn admin_force_cancel_gagal_jastiper_403() {
    setup_jwt_secret();

    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockInventoryClient::new(),
        MockWalletClient::new(),
    ));
    let token = make_test_token(jastiper_id, "JASTIPER");
    let req = json_request(
        "POST",
        &format!("/admin/orders/{}/force-cancel", order_id),
        &token,
        Some(cancel_body()),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn admin_force_cancel_gagal_order_tidak_ditemukan_404() {
    setup_jwt_secret();

    let admin_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

    let app = TestApp::new(default_state(
        repo,
        MockInventoryClient::new(),
        MockWalletClient::new(),
    ));
    let token = make_test_token(admin_id, "ADMIN");
    let req = json_request(
        "POST",
        &format!("/admin/orders/{}/force-cancel", order_id),
        &token,
        Some(cancel_body()),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn admin_force_cancel_gagal_validasi_body_kosong_422() {
    setup_jwt_secret();

    let admin_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockInventoryClient::new(),
        MockWalletClient::new(),
    ));
    let token = make_test_token(admin_id, "ADMIN");

    // Body kosong atau tidak ada field cancellation_reason
    let invalid_body = json!({});
    let req = json_request(
        "POST",
        &format!("/admin/orders/{}/force-cancel", order_id),
        &token,
        Some(invalid_body),
    );
    let (status, _) = app.send(req).await;

    // JSON parse error → 422 atau 400
    assert!(
        status == StatusCode::UNPROCESSABLE_ENTITY || status == StatusCode::BAD_REQUEST,
        "Expected 422 or 400, got {}",
        status
    );
}

#[tokio::test]
async fn admin_force_cancel_sukses_meski_release_stock_gagal() {
    setup_jwt_secret();

    // release_stock dan refund_wallet error diabaikan (best-effort) di force_cancel
    let admin_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut inv = MockInventoryClient::new();
    let mut wallet = MockWalletClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    let cancelled = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);

    let order_clone = order.clone();
    let cancelled_clone = cancelled.clone();
    let mut call_count = 0;
    repo.expect_find_by_id().returning(move |_| {
        call_count += 1;
        if call_count == 1 {
            Ok(Some(order_clone.clone()))
        } else {
            Ok(Some(cancelled_clone.clone()))
        }
    });
    repo.expect_update()
        .returning(move |_, _, _| Ok(cancelled.clone()));

    // Simulasikan inventory error → tapi force_cancel masih sukses
    inv.expect_release_stock()
        .returning(|_, _, _| Err(AppError::Internal));
    wallet
        .expect_refund_wallet()
        .returning(|_, _, _, _| Err(AppError::Internal));

    let app = TestApp::new(default_state(repo, inv, wallet));
    let token = make_test_token(admin_id, "ADMIN");
    let req = json_request(
        "POST",
        &format!("/admin/orders/{}/force-cancel", order_id),
        &token,
        Some(cancel_body()),
    );
    let (status, body) = app.send(req).await;

    // force_cancel mengabaikan error dari release_stock dan refund_wallet
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn admin_force_cancel_sukses_order_pending() {
    setup_jwt_secret();

    // Order Pending juga bisa di-force-cancel (tidak perlu refund karena belum bayar)
    let admin_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut inv = MockInventoryClient::new();
    let mut wallet = MockWalletClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    let cancelled = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);

    let order_clone = order.clone();
    let cancelled_clone = cancelled.clone();
    let mut call_count = 0;
    repo.expect_find_by_id().returning(move |_| {
        call_count += 1;
        if call_count == 1 {
            Ok(Some(order_clone.clone()))
        } else {
            Ok(Some(cancelled_clone.clone()))
        }
    });
    repo.expect_update()
        .returning(move |_, _, _| Ok(cancelled.clone()));

    inv.expect_release_stock().returning(|_, _, _| Ok(()));
    wallet.expect_refund_wallet().returning(|_, _, _, _| {
        Ok(RefundResponse {
            transaction_id: "".to_string(),
        })
    });

    let app = TestApp::new(default_state(repo, inv, wallet));
    let token = make_test_token(admin_id, "ADMIN");
    let req = json_request(
        "POST",
        &format!("/admin/orders/{}/force-cancel", order_id),
        &token,
        Some(cancel_body()),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
}
