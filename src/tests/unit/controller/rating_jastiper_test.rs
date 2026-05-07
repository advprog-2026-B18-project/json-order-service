use axum::http::StatusCode;
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::order::{Order, OrderStatus};
use crate::models::rating_jastiper::RatingJastiper;
use crate::repositories::order_repository::MockOrderRepository;
use crate::repositories::order_status_history_repository::MockOrderStatusHistoryRepository;
use crate::repositories::rating_jastiper_repository::MockRatingJastiperRepository;
use crate::repositories::rating_product_repository::MockRatingProductRepository;
use crate::services::auth_client::MockAuthClient;
use crate::services::inventory_client::MockInventoryClient;
use crate::services::wallet_client::MockWalletClient;
use crate::state::AppState;
use crate::tests::unit::controller::helper_test::{TestApp, json_request, make_test_token};

// ──────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────

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
    }
}

fn make_rating_jastiper(order_id: Uuid, titipers_id: Uuid) -> RatingJastiper {
    RatingJastiper {
        rating_jastiper_id: Uuid::new_v4(),
        order_id,
        titipers_id,
        jastiper_rating: 5.0,
        jastiper_review: Some("Jastiper terpercaya".to_string()),
        created_at: Utc::now(),
    }
}

fn default_state(
    order_repo: MockOrderRepository,
    rating_jastiper_repo: MockRatingJastiperRepository,
) -> AppState {
    AppState {
        order_repo: Arc::new(order_repo),
        inventory_client: Arc::new(MockInventoryClient::new()),
        wallet_client: Arc::new(MockWalletClient::new()),
        order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
        rating_product_repo: Arc::new(MockRatingProductRepository::new()),
        rating_jastiper_repo: Arc::new(rating_jastiper_repo),
        auth_client: Arc::new(MockAuthClient::new()),
    }
}

fn valid_rating_body() -> serde_json::Value {
    json!({
        "jastiper_rating": 5.0,
        "jastiper_review": "Jastiper sangat cepat dan terpercaya"
    })
}

// ──────────────────────────────────────────────────────────────
// POST /orders/{order_id}/rating/jastiper
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn submit_rating_jastiper_sukses_201() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingJastiperRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let rating = make_rating_jastiper(order_id, titipers_id);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    rating_repo
        .expect_find_by_order_id()
        .returning(|_| Ok(None));
    let rating_clone = rating.clone();
    rating_repo
        .expect_create()
        .returning(move |_, _, _| Ok(rating_clone.clone()));

    let app = TestApp::new(default_state(repo, rating_repo));
    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "POST",
        &format!("/orders/{}/rating/jastiper", order_id),
        &token,
        Some(valid_rating_body()),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["success"], true);
    assert_eq!(body["message"], "Rating berhasil dikirim");
    assert!(body["data"]["rating_id"].is_string());
    assert!(body["data"]["order_id"].is_string());
    assert!(body["data"]["jastiper_rating"].is_number());
    assert!(body["data"]["created_at"].is_string());
    // Pastikan tidak ada field product_rating
    assert!(body["data"]["product_rating"].is_null());
}

#[tokio::test]
async fn submit_rating_jastiper_gagal_unauthorized_401() {
    setup_jwt_secret();

    let order_id = Uuid::new_v4();
    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockRatingJastiperRepository::new(),
    ));

    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/orders/{}/rating/jastiper", order_id))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(valid_rating_body().to_string()))
        .unwrap();

    let (status, _) = app.send(req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn submit_rating_jastiper_gagal_order_tidak_ditemukan_404() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

    let app = TestApp::new(default_state(repo, MockRatingJastiperRepository::new()));
    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "POST",
        &format!("/orders/{}/rating/jastiper", order_id),
        &token,
        Some(valid_rating_body()),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn submit_rating_jastiper_gagal_bukan_titipers_pemilik_403() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let app = TestApp::new(default_state(repo, MockRatingJastiperRepository::new()));
    let token = make_test_token(orang_lain, "TITIPERS");
    let req = json_request(
        "POST",
        &format!("/orders/{}/rating/jastiper", order_id),
        &token,
        Some(valid_rating_body()),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn submit_rating_jastiper_gagal_order_belum_completed_422() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    // Status Paid → belum Completed
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let app = TestApp::new(default_state(repo, MockRatingJastiperRepository::new()));
    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "POST",
        &format!("/orders/{}/rating/jastiper", order_id),
        &token,
        Some(valid_rating_body()),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn submit_rating_jastiper_gagal_rating_sudah_ada_409() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingJastiperRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let existing = make_rating_jastiper(order_id, titipers_id);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    rating_repo
        .expect_find_by_order_id()
        .returning(move |_| Ok(Some(existing.clone())));

    let app = TestApp::new(default_state(repo, rating_repo));
    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "POST",
        &format!("/orders/{}/rating/jastiper", order_id),
        &token,
        Some(valid_rating_body()),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn submit_rating_jastiper_gagal_rating_nol_400() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockRatingJastiperRepository::new(),
    ));
    let token = make_test_token(titipers_id, "TITIPERS");

    let invalid_body = json!({
        "jastiper_rating": 0.0,
        "jastiper_review": null
    });
    let req = json_request(
        "POST",
        &format!("/orders/{}/rating/jastiper", order_id),
        &token,
        Some(invalid_body),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn submit_rating_jastiper_gagal_rating_di_atas_5_400() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockRatingJastiperRepository::new(),
    ));
    let token = make_test_token(titipers_id, "TITIPERS");

    let invalid_body = json!({
        "jastiper_rating": 10.0,
        "jastiper_review": null
    });
    let req = json_request(
        "POST",
        &format!("/orders/{}/rating/jastiper", order_id),
        &token,
        Some(invalid_body),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn submit_rating_jastiper_sukses_tanpa_review() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingJastiperRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let rating = RatingJastiper {
        rating_jastiper_id: Uuid::new_v4(),
        order_id,
        titipers_id,
        jastiper_rating: 3.0,
        jastiper_review: None,
        created_at: Utc::now(),
    };

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    rating_repo
        .expect_find_by_order_id()
        .returning(|_| Ok(None));
    let rating_clone = rating.clone();
    rating_repo
        .expect_create()
        .returning(move |_, _, _| Ok(rating_clone.clone()));

    let app = TestApp::new(default_state(repo, rating_repo));
    let token = make_test_token(titipers_id, "TITIPERS");
    let body = json!({
        "jastiper_rating": 3.0,
        "jastiper_review": null
    });
    let req = json_request(
        "POST",
        &format!("/orders/{}/rating/jastiper", order_id),
        &token,
        Some(body),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["success"], true);
}

// ──────────────────────────────────────────────────────────────
// GET /orders/{order_id}/rating/jastiper
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_rating_jastiper_sukses_200_sebagai_titipers() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingJastiperRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let rating = make_rating_jastiper(order_id, titipers_id);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    let rating_clone = rating.clone();
    rating_repo
        .expect_find_by_order_id()
        .returning(move |_| Ok(Some(rating_clone.clone())));

    let app = TestApp::new(default_state(repo, rating_repo));
    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "GET",
        &format!("/orders/{}/rating/jastiper", order_id),
        &token,
        None,
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["message"], "Rating ditemukan");
    assert!(body["data"].is_object());
    assert!(body["data"]["jastiper_rating"].is_number());
    // Pastikan tidak ada field product_rating
    assert!(body["data"]["product_rating"].is_null());
}

#[tokio::test]
async fn get_rating_jastiper_sukses_200_sebagai_jastiper() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingJastiperRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let rating = make_rating_jastiper(order_id, titipers_id);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    let rating_clone = rating.clone();
    rating_repo
        .expect_find_by_order_id()
        .returning(move |_| Ok(Some(rating_clone.clone())));

    let app = TestApp::new(default_state(repo, rating_repo));
    // Jastiper juga boleh lihat rating yang diterima
    let token = make_test_token(jastiper_id, "JASTIPER");
    let req = json_request(
        "GET",
        &format!("/orders/{}/rating/jastiper", order_id),
        &token,
        None,
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn get_rating_jastiper_gagal_unauthorized_401() {
    setup_jwt_secret();

    let order_id = Uuid::new_v4();
    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockRatingJastiperRepository::new(),
    ));

    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/orders/{}/rating/jastiper", order_id))
        .body(axum::body::Body::empty())
        .unwrap();

    let (status, _) = app.send(req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_rating_jastiper_gagal_order_tidak_ditemukan_404() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

    let app = TestApp::new(default_state(repo, MockRatingJastiperRepository::new()));
    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "GET",
        &format!("/orders/{}/rating/jastiper", order_id),
        &token,
        None,
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn get_rating_jastiper_gagal_bukan_pemilik_403() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let app = TestApp::new(default_state(repo, MockRatingJastiperRepository::new()));
    let token = make_test_token(orang_lain, "TITIPERS");
    let req = json_request(
        "GET",
        &format!("/orders/{}/rating/jastiper", order_id),
        &token,
        None,
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn get_rating_jastiper_gagal_rating_belum_ada_404() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingJastiperRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    rating_repo
        .expect_find_by_order_id()
        .returning(|_| Ok(None));

    let app = TestApp::new(default_state(repo, rating_repo));
    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "GET",
        &format!("/orders/{}/rating/jastiper", order_id),
        &token,
        None,
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["success"], false);
}
