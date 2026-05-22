use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::filter_pagination::PaginationParams;
use crate::models::order::{Order, OrderStatus};
use crate::models::rating_jastiper::{CreateRatingJastiperRequest, RatingJastiper};
use crate::repositories::order_repository::MockOrderRepository;
use crate::repositories::rating_jastiper_repository::MockRatingJastiperRepository;
use crate::services::rating_jastiper::{
    get_rating, get_ratings_by_jastiper, submit_rating_jastiper,
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
        product_snapshot: serde_json::json!({}),
        quantity: 1,
        unit_price: 10_000,
        service_fee: 1_000,
        total_price: 11_000,
        status,
        shipping_address: serde_json::json!({}),
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

fn make_rating_jastiper(order_id: Uuid, titipers_id: Uuid) -> RatingJastiper {
    RatingJastiper {
        rating_jastiper_id: Uuid::new_v4(),
        order_id,
        titipers_id,
        jastiper_rating: 5.0,
        jastiper_review: Some("Jastiper sangat responsif".to_string()),
        created_at: Utc::now(),
    }
}

fn valid_request() -> CreateRatingJastiperRequest {
    CreateRatingJastiperRequest {
        jastiper_rating: 5.0,
        jastiper_review: Some("Jastiper sangat responsif".to_string()),
    }
}

// ──────────────────────────────────────────────────────────────
// submit_rating_jastiper
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn submit_rating_jastiper_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingJastiperRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let rating = make_rating_jastiper(order_id, titipers_id);
    let rating_clone = rating.clone();

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    rating_repo
        .expect_find_by_order_id()
        .returning(|_| Ok(None));
    rating_repo
        .expect_create()
        .returning(move |_, _, _| Ok(rating_clone.clone()));

    let result = submit_rating_jastiper(
        Arc::new(order_repo),
        Arc::new(rating_repo),
        order_id,
        titipers_id,
        valid_request(),
    )
    .await;

    assert!(result.is_ok());
    let r = result.unwrap();
    assert_eq!(r.order_id, order_id);
    assert_eq!(r.titipers_id, titipers_id);
    assert_eq!(r.jastiper_rating, 5.0);
}

#[tokio::test]
async fn submit_rating_jastiper_gagal_validasi_rating_nol() {
    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let result = submit_rating_jastiper(
        Arc::new(MockOrderRepository::new()),
        Arc::new(MockRatingJastiperRepository::new()),
        order_id,
        titipers_id,
        CreateRatingJastiperRequest {
            jastiper_rating: 0.0,
            jastiper_review: None,
        },
    )
    .await;

    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[tokio::test]
async fn submit_rating_jastiper_gagal_validasi_rating_di_atas_5() {
    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let result = submit_rating_jastiper(
        Arc::new(MockOrderRepository::new()),
        Arc::new(MockRatingJastiperRepository::new()),
        order_id,
        titipers_id,
        CreateRatingJastiperRequest {
            jastiper_rating: 6.0,
            jastiper_review: None,
        },
    )
    .await;

    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[tokio::test]
async fn submit_rating_jastiper_gagal_order_tidak_ditemukan() {
    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    order_repo.expect_find_by_id().returning(|_| Ok(None));

    let result = submit_rating_jastiper(
        Arc::new(order_repo),
        Arc::new(MockRatingJastiperRepository::new()),
        order_id,
        titipers_id,
        valid_request(),
    )
    .await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn submit_rating_jastiper_gagal_bukan_titipers_pemilik() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let result = submit_rating_jastiper(
        Arc::new(order_repo),
        Arc::new(MockRatingJastiperRepository::new()),
        order_id,
        orang_lain,
        valid_request(),
    )
    .await;

    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn submit_rating_jastiper_gagal_order_belum_completed() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);
    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let result = submit_rating_jastiper(
        Arc::new(order_repo),
        Arc::new(MockRatingJastiperRepository::new()),
        order_id,
        titipers_id,
        valid_request(),
    )
    .await;

    assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
}

#[tokio::test]
async fn submit_rating_jastiper_gagal_rating_sudah_ada() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingJastiperRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let existing = make_rating_jastiper(order_id, titipers_id);

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    rating_repo
        .expect_find_by_order_id()
        .returning(move |_| Ok(Some(existing.clone())));

    let result = submit_rating_jastiper(
        Arc::new(order_repo),
        Arc::new(rating_repo),
        order_id,
        titipers_id,
        valid_request(),
    )
    .await;

    assert!(matches!(result, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn submit_rating_jastiper_gagal_db_error() {
    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    order_repo
        .expect_find_by_id()
        .returning(|_| Err(AppError::Internal));

    let result = submit_rating_jastiper(
        Arc::new(order_repo),
        Arc::new(MockRatingJastiperRepository::new()),
        order_id,
        titipers_id,
        valid_request(),
    )
    .await;

    assert!(result.is_err());
}

// ──────────────────────────────────────────────────────────────
// get_rating
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_rating_jastiper_sukses_sebagai_titipers() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingJastiperRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let rating = make_rating_jastiper(order_id, titipers_id);
    let rating_clone = rating.clone();

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    rating_repo
        .expect_find_by_order_id()
        .returning(move |_| Ok(Some(rating_clone.clone())));

    let result = get_rating(
        Arc::new(order_repo),
        Arc::new(rating_repo),
        order_id,
        titipers_id,
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().order_id, order_id);
}

#[tokio::test]
async fn get_rating_jastiper_sukses_sebagai_jastiper() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingJastiperRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let rating = make_rating_jastiper(order_id, titipers_id);
    let rating_clone = rating.clone();

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    rating_repo
        .expect_find_by_order_id()
        .returning(move |_| Ok(Some(rating_clone.clone())));

    let result = get_rating(
        Arc::new(order_repo),
        Arc::new(rating_repo),
        order_id,
        jastiper_id,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn get_rating_jastiper_gagal_order_tidak_ditemukan() {
    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    order_repo.expect_find_by_id().returning(|_| Ok(None));

    let result = get_rating(
        Arc::new(order_repo),
        Arc::new(MockRatingJastiperRepository::new()),
        order_id,
        titipers_id,
    )
    .await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn get_rating_jastiper_gagal_bukan_pemilik() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let result = get_rating(
        Arc::new(order_repo),
        Arc::new(MockRatingJastiperRepository::new()),
        order_id,
        orang_lain,
    )
    .await;

    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn get_rating_jastiper_gagal_rating_belum_ada() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingJastiperRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    rating_repo
        .expect_find_by_order_id()
        .returning(|_| Ok(None));

    let result = get_rating(
        Arc::new(order_repo),
        Arc::new(rating_repo),
        order_id,
        titipers_id,
    )
    .await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

// === Error Path: repo::create fails ===

#[tokio::test]
async fn submit_rating_jastiper_gagal_repo_create_error() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingJastiperRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    rating_repo
        .expect_find_by_order_id()
        .returning(|_| Ok(None));
    rating_repo
        .expect_create()
        .returning(|_, _, _| Err(AppError::Internal));

    let result = submit_rating_jastiper(
        Arc::new(order_repo),
        Arc::new(rating_repo),
        order_id,
        titipers_id,
        valid_request(),
    )
    .await;

    assert!(matches!(result, Err(AppError::Internal)));
}

// === Notification success path ===

#[serial_test::serial]
#[tokio::test]
async fn submit_rating_jastiper_notification_success_logs_info() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mock_server = wiremock::MockServer::start().await;
    unsafe {
        std::env::set_var("USER_SERVICE_URL", mock_server.uri());
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }

    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path_regex(r"/internal/users/.+/rating"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&mock_server)
        .await;

    let mut order_repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingJastiperRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let rating = make_rating_jastiper(order_id, titipers_id);
    let rating_clone = rating.clone();

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    rating_repo
        .expect_find_by_order_id()
        .returning(|_| Ok(None));
    rating_repo
        .expect_create()
        .returning(move |_, _, _| Ok(rating_clone.clone()));

    let result = submit_rating_jastiper(
        Arc::new(order_repo),
        Arc::new(rating_repo),
        order_id,
        titipers_id,
        valid_request(),
    )
    .await;

    assert!(result.is_ok());
}

// ──────────────────────────────────────────────────────────────
// get_ratings_by_jastiper
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_ratings_by_jastiper_ada_rating_200_dengan_list_dan_average() {
    let jastiper_id = Uuid::new_v4();
    let mut rating_repo = MockRatingJastiperRepository::new();

    let rating1 = RatingJastiper {
        rating_jastiper_id: Uuid::new_v4(),
        order_id: Uuid::new_v4(),
        titipers_id: Uuid::new_v4(),
        jastiper_rating: 3.0,
        jastiper_review: None,
        created_at: Utc::now(),
    };
    let rating2 = RatingJastiper {
        rating_jastiper_id: Uuid::new_v4(),
        order_id: Uuid::new_v4(),
        titipers_id: Uuid::new_v4(),
        jastiper_rating: 5.0,
        jastiper_review: None,
        created_at: Utc::now(),
    };
    let ratings = vec![rating1, rating2];
    let ratings_clone = ratings.clone();

    rating_repo
        .expect_find_all_by_jastiper_id()
        .returning(move |_, _| Ok((ratings_clone.clone(), 2)));

    let params = PaginationParams::default();
    let result = get_ratings_by_jastiper(Arc::new(rating_repo), jastiper_id, &params).await;

    assert!(result.is_ok());
    let (data, total, avg) = result.unwrap();
    assert_eq!(data.len(), 2);
    assert_eq!(total, 2);
    assert_eq!(avg, 4.0);
}

#[tokio::test]
async fn get_ratings_by_jastiper_tidak_ada_rating_200_dengan_list_kosong() {
    let jastiper_id = Uuid::new_v4();
    let mut rating_repo = MockRatingJastiperRepository::new();

    rating_repo
        .expect_find_all_by_jastiper_id()
        .returning(|_, _| Ok((vec![], 0)));

    let params = PaginationParams::default();
    let result = get_ratings_by_jastiper(Arc::new(rating_repo), jastiper_id, &params).await;

    assert!(result.is_ok());
    let (data, total, avg) = result.unwrap();
    assert!(data.is_empty());
    assert_eq!(total, 0);
    assert_eq!(avg, 0.0);
}
