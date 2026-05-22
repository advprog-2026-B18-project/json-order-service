use axum::http::StatusCode;
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use std::sync::Once;
use uuid::Uuid;

static INIT: Once = Once::new();

use crate::models::order::{Order, OrderStatus};
use crate::models::rating_product::RatingProduct;
use crate::repositories::order_repository::MockOrderRepository;
use crate::repositories::order_status_history_repository::MockOrderStatusHistoryRepository;
use crate::repositories::rating_jastiper_repository::MockRatingJastiperRepository;
use crate::repositories::rating_product_repository::MockRatingProductRepository;
use crate::services::auth_client::MockAuthClient;
use crate::services::inventory_client::MockInventoryClient;
use crate::services::wallet_client::MockWalletClient;
use crate::state::AppState;
use crate::tests::unit::controller::helper_test::{
    TestApp, dummy_mq_pool, json_request, make_test_token, noop_checkout_publisher,
    noop_idempotency_repo,
};

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

fn make_rating_product(order_id: Uuid, titipers_id: Uuid) -> RatingProduct {
    RatingProduct {
        rating_product_id: Uuid::new_v4(),
        order_id,
        titipers_id,
        product_rating: 5.0,
        product_review: Some("Produk bagus sekali".to_string()),
        product_images: vec![],
        created_at: Utc::now(),
    }
}

fn default_state(
    order_repo: MockOrderRepository,
    rating_product_repo: MockRatingProductRepository,
) -> AppState {
    AppState {
        order_repo: Arc::new(order_repo),
        inventory_client: Arc::new(MockInventoryClient::new()),
        wallet_client: Arc::new(MockWalletClient::new()),
        order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
        rating_product_repo: Arc::new(rating_product_repo),
        rating_jastiper_repo: Arc::new(MockRatingJastiperRepository::new()),
        auth_client: Arc::new(MockAuthClient::new()),
        checkout_publisher: Arc::new(noop_checkout_publisher()),
        mq_pool: dummy_mq_pool(),
        idempotency_repo: Arc::new(noop_idempotency_repo()),
    }
}

fn valid_rating_body() -> serde_json::Value {
    json!({
        "product_rating": 5.0,
        "product_review": "Produk sangat bagus",
        "product_images": []
    })
}

// ──────────────────────────────────────────────────────────────
// POST /orders/{order_id}/rating/product
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn submit_rating_product_sukses_201() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingProductRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let rating = make_rating_product(order_id, titipers_id);

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
        &format!("/orders/{}/rating/product", order_id),
        &token,
        Some(valid_rating_body()),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["success"], true);
    assert_eq!(body["message"], "Rating berhasil dikirim");
    assert!(body["data"]["rating_id"].is_string());
    assert!(body["data"]["order_id"].is_string());
    assert!(body["data"]["product_rating"].is_number());
    assert!(body["data"]["created_at"].is_string());
}

#[tokio::test]
async fn submit_rating_product_gagal_unauthorized_401() {
    setup_jwt_secret();

    let order_id = Uuid::new_v4();
    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockRatingProductRepository::new(),
    ));

    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/orders/{}/rating/product", order_id))
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(valid_rating_body().to_string()))
        .unwrap();

    let (status, _) = app.send(req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn submit_rating_product_gagal_order_tidak_ditemukan_404() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

    let app = TestApp::new(default_state(repo, MockRatingProductRepository::new()));
    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "GET",
        &format!("/orders/{}/rating/product", order_id),
        &token,
        None,
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["success"], false);
}

// ──────────────────────────────────────────────────────────────
// GET /products/{product_id}/ratings (public — no auth)
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_ratings_by_product_sukses_200() {
    setup_jwt_secret();
    let product_id = Uuid::new_v4();

    let mut rating_repo = MockRatingProductRepository::new();
    let ratings = vec![make_rating_product(Uuid::new_v4(), Uuid::new_v4())];
    rating_repo
        .expect_find_all_by_product_id()
        .returning(move |_, _| Ok((ratings.clone(), 1i64)));

    let app = TestApp::new(default_state(MockOrderRepository::new(), rating_repo));
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/products/{}/ratings", product_id))
        .body(axum::body::Body::empty())
        .unwrap();
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert!(body["data"]["ratings"].is_array());
    assert_eq!(body["data"]["total"], 1);
}

#[tokio::test]
async fn get_ratings_by_product_empty_200() {
    setup_jwt_secret();
    let product_id = Uuid::new_v4();

    let mut rating_repo = MockRatingProductRepository::new();
    rating_repo
        .expect_find_all_by_product_id()
        .returning(move |_, _| Ok((vec![], 0i64)));

    let app = TestApp::new(default_state(MockOrderRepository::new(), rating_repo));
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/products/{}/ratings", product_id))
        .body(axum::body::Body::empty())
        .unwrap();
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert!(body["data"]["ratings"].as_array().unwrap().is_empty());
    assert_eq!(body["data"]["total"], 0);
}

#[tokio::test]
async fn submit_rating_product_gagal_bukan_pemilik_403() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let app = TestApp::new(default_state(repo, MockRatingProductRepository::new()));
    let token = make_test_token(orang_lain, "TITIPERS");
    let req = json_request(
        "POST",
        &format!("/orders/{}/rating/product", order_id),
        &token,
        Some(valid_rating_body()),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn submit_rating_product_gagal_order_belum_completed_422() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    // Status Shipped → belum Completed
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let app = TestApp::new(default_state(repo, MockRatingProductRepository::new()));
    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "POST",
        &format!("/orders/{}/rating/product", order_id),
        &token,
        Some(valid_rating_body()),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn submit_rating_product_gagal_rating_sudah_ada_409() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingProductRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let existing_rating = make_rating_product(order_id, titipers_id);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    rating_repo
        .expect_find_by_order_id()
        .returning(move |_| Ok(Some(existing_rating.clone())));

    let app = TestApp::new(default_state(repo, rating_repo));
    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "POST",
        &format!("/orders/{}/rating/product", order_id),
        &token,
        Some(valid_rating_body()),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn submit_rating_product_gagal_validasi_rating_di_luar_range_422() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockRatingProductRepository::new(),
    ));
    let token = make_test_token(titipers_id, "TITIPERS");

    // rating = 0 → di bawah minimum 1.0
    let invalid_body = json!({
        "product_rating": 0.0,
        "product_review": null,
        "product_images": null
    });
    let req = json_request(
        "POST",
        &format!("/orders/{}/rating/product", order_id),
        &token,
        Some(invalid_body),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn submit_rating_product_gagal_rating_di_atas_5_422() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockRatingProductRepository::new(),
    ));
    let token = make_test_token(titipers_id, "TITIPERS");

    let invalid_body = json!({
        "product_rating": 6.0,
        "product_review": null,
        "product_images": null
    });
    let req = json_request(
        "POST",
        &format!("/orders/{}/rating/product", order_id),
        &token,
        Some(invalid_body),
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["success"], false);
}

// ──────────────────────────────────────────────────────────────
// GET /orders/{order_id}/rating/product
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_rating_product_sukses_200_sebagai_titipers() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingProductRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let rating = make_rating_product(order_id, titipers_id);

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
        &format!("/orders/{}/rating/product", order_id),
        &token,
        None,
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
    assert_eq!(body["message"], "Rating ditemukan");
    assert!(body["data"].is_object());
}

#[tokio::test]
async fn get_rating_product_sukses_200_sebagai_jastiper() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingProductRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let rating = make_rating_product(order_id, titipers_id);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    let rating_clone = rating.clone();
    rating_repo
        .expect_find_by_order_id()
        .returning(move |_| Ok(Some(rating_clone.clone())));

    let app = TestApp::new(default_state(repo, rating_repo));
    // Jastiper juga boleh lihat rating produknya
    let token = make_test_token(jastiper_id, "JASTIPER");
    let req = json_request(
        "GET",
        &format!("/orders/{}/rating/product", order_id),
        &token,
        None,
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], true);
}

#[tokio::test]
async fn get_rating_product_gagal_unauthorized_401() {
    setup_jwt_secret();

    let order_id = Uuid::new_v4();
    let app = TestApp::new(default_state(
        MockOrderRepository::new(),
        MockRatingProductRepository::new(),
    ));

    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/orders/{}/rating/product", order_id))
        .body(axum::body::Body::empty())
        .unwrap();

    let (status, _) = app.send(req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_rating_product_gagal_order_tidak_ditemukan_404() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

    let app = TestApp::new(default_state(repo, MockRatingProductRepository::new()));
    let token = make_test_token(titipers_id, "TITIPERS");
    let req = json_request(
        "GET",
        &format!("/orders/{}/rating/product", order_id),
        &token,
        None,
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn get_rating_product_gagal_bukan_pemilik_403() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let app = TestApp::new(default_state(repo, MockRatingProductRepository::new()));
    let token = make_test_token(orang_lain, "TITIPERS");
    let req = json_request(
        "GET",
        &format!("/orders/{}/rating/product", order_id),
        &token,
        None,
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn get_rating_product_gagal_rating_belum_ada_404() {
    setup_jwt_secret();

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingProductRepository::new();

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
        &format!("/orders/{}/rating/product", order_id),
        &token,
        None,
    );
    let (status, body) = app.send(req).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["success"], false);
}
