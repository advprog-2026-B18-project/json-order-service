use uuid::Uuid;

use crate::error::AppError;
use crate::models::order_state::OrderStatus;
use crate::models::rating_jastiper::{CreateRatingJastiperRequest, RatingJastiper};
use crate::models::rating_product::{CreateRatingProductRequest, RatingProduct};
use crate::ports::order_repository::MockOrderRepository;
use crate::ports::rating_jastiper_repository::MockRatingJastiperRepository;
use crate::ports::rating_product_repository::MockRatingProductRepository;
use crate::services::{rating_jastiper, rating_product};

use serde_json::json;

// ─── Helpers ────────────────────────────────────────────────────────────────

fn make_order(
    order_id: Uuid,
    titipers_id: Uuid,
    jastiper_id: Uuid,
    status: OrderStatus,
) -> crate::models::order::Order {
    crate::models::order::Order {
        order_id,
        titipers_id,
        jastiper_id,
        product_id: Uuid::new_v4(),
        product_snapshot: json!({ "product_id": Uuid::new_v4() }),
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
    }
}

fn make_rating_jastiper_request() -> CreateRatingJastiperRequest {
    CreateRatingJastiperRequest {
        jastiper_rating: 5.0,
        jastiper_review: Some("Jastiper sangat responsif".to_string()),
    }
}

fn make_rating_product_request() -> CreateRatingProductRequest {
    CreateRatingProductRequest {
        product_rating: 4.5,
        product_review: Some("Produk sesuai deskripsi".to_string()),
        product_images: Some(vec!["https://img.example.com/1.jpg".to_string()]),
    }
}

fn make_rating_jastiper(order_id: Uuid, titipers_id: Uuid) -> RatingJastiper {
    RatingJastiper {
        rating_jastiper_id: Uuid::new_v4(),
        order_id,
        titipers_id,
        jastiper_rating: 5.0,
        jastiper_review: Some("Bagus".to_string()),
        created_at: chrono::Utc::now(),
    }
}

fn make_rating_product(order_id: Uuid, titipers_id: Uuid) -> RatingProduct {
    RatingProduct {
        rating_product_id: Uuid::new_v4(),
        order_id,
        titipers_id,
        product_rating: 4.5,
        product_review: Some("Sesuai deskripsi".to_string()),
        product_images: Vec::new(),
        created_at: chrono::Utc::now(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// rating_jastiper::submit_rating_jastiper
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn submit_rating_jastiper_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingJastiperRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let expected_rating = make_rating_jastiper(order_id, titipers_id);

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    rating_repo
        .expect_find_by_order_id()
        .returning(|_| Ok(None));

    rating_repo
        .expect_create()
        .returning(move |_, _, _| Ok(expected_rating.clone()));

    let req = make_rating_jastiper_request();
    let result = rating_jastiper::submit_rating_jastiper(
        &order_repo,
        &rating_repo,
        order_id,
        titipers_id,
        req,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn submit_rating_jastiper_gagal_order_tidak_ditemukan() {
    let mut order_repo = MockOrderRepository::new();
    let rating_repo = MockRatingJastiperRepository::new();

    order_repo.expect_find_by_id().returning(|_| Ok(None));

    let req = make_rating_jastiper_request();
    let result = rating_jastiper::submit_rating_jastiper(
        &order_repo,
        &rating_repo,
        Uuid::new_v4(),
        Uuid::new_v4(),
        req,
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
    let rating_repo = MockRatingJastiperRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = make_rating_jastiper_request();
    let result = rating_jastiper::submit_rating_jastiper(
        &order_repo,
        &rating_repo,
        order_id,
        orang_lain,
        req,
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
    let rating_repo = MockRatingJastiperRepository::new();

    // Status bukan Completed, misal Shipped
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = make_rating_jastiper_request();
    let result = rating_jastiper::submit_rating_jastiper(
        &order_repo,
        &rating_repo,
        order_id,
        titipers_id,
        req,
    )
    .await;

    assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
}

#[tokio::test]
async fn submit_rating_jastiper_gagal_sudah_pernah_rating() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingJastiperRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let existing_rating = make_rating_jastiper(order_id, titipers_id);

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    rating_repo
        .expect_find_by_order_id()
        .returning(move |_| Ok(Some(existing_rating.clone())));

    let req = make_rating_jastiper_request();
    let result = rating_jastiper::submit_rating_jastiper(
        &order_repo,
        &rating_repo,
        order_id,
        titipers_id,
        req,
    )
    .await;

    assert!(matches!(result, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn submit_rating_jastiper_gagal_db_error_saat_create() {
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

    let req = make_rating_jastiper_request();
    let result = rating_jastiper::submit_rating_jastiper(
        &order_repo,
        &rating_repo,
        order_id,
        titipers_id,
        req,
    )
    .await;

    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════════════════
// rating_jastiper::get_rating
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn get_rating_jastiper_sukses_sebagai_titipers() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingJastiperRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let rating = make_rating_jastiper(order_id, titipers_id);

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    rating_repo
        .expect_find_by_order_id()
        .returning(move |_| Ok(Some(rating.clone())));

    let result =
        rating_jastiper::get_rating(&order_repo, &rating_repo, order_id, titipers_id).await;
    assert!(result.is_ok());
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

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    rating_repo
        .expect_find_by_order_id()
        .returning(move |_| Ok(Some(rating.clone())));

    let result =
        rating_jastiper::get_rating(&order_repo, &rating_repo, order_id, jastiper_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn get_rating_jastiper_gagal_order_tidak_ditemukan() {
    let mut order_repo = MockOrderRepository::new();
    let rating_repo = MockRatingJastiperRepository::new();

    order_repo.expect_find_by_id().returning(|_| Ok(None));

    let result =
        rating_jastiper::get_rating(&order_repo, &rating_repo, Uuid::new_v4(), Uuid::new_v4())
            .await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn get_rating_jastiper_gagal_bukan_pemilik_order() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let rating_repo = MockRatingJastiperRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let result = rating_jastiper::get_rating(&order_repo, &rating_repo, order_id, orang_lain).await;

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

    let result =
        rating_jastiper::get_rating(&order_repo, &rating_repo, order_id, titipers_id).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

// ═══════════════════════════════════════════════════════════════════════════
// rating_product::submit_rating
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn submit_rating_product_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingProductRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let expected_rating = make_rating_product(order_id, titipers_id);

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    rating_repo
        .expect_find_by_order_id()
        .returning(|_| Ok(None));

    rating_repo
        .expect_create()
        .returning(move |_, _, _| Ok(expected_rating.clone()));

    let req = make_rating_product_request();
    let result =
        rating_product::submit_rating(&order_repo, &rating_repo, order_id, titipers_id, req).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn submit_rating_product_gagal_order_tidak_ditemukan() {
    let mut order_repo = MockOrderRepository::new();
    let rating_repo = MockRatingProductRepository::new();

    order_repo.expect_find_by_id().returning(|_| Ok(None));

    let req = make_rating_product_request();
    let result = rating_product::submit_rating(
        &order_repo,
        &rating_repo,
        Uuid::new_v4(),
        Uuid::new_v4(),
        req,
    )
    .await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn submit_rating_product_gagal_bukan_titipers_pemilik() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let rating_repo = MockRatingProductRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = make_rating_product_request();
    let result =
        rating_product::submit_rating(&order_repo, &rating_repo, order_id, orang_lain, req).await;

    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn submit_rating_product_gagal_order_belum_completed() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let rating_repo = MockRatingProductRepository::new();

    // Status Paid bukan Completed
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = make_rating_product_request();
    let result =
        rating_product::submit_rating(&order_repo, &rating_repo, order_id, titipers_id, req).await;

    assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
}

#[tokio::test]
async fn submit_rating_product_gagal_sudah_pernah_rating() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingProductRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let existing = make_rating_product(order_id, titipers_id);

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    rating_repo
        .expect_find_by_order_id()
        .returning(move |_| Ok(Some(existing.clone())));

    let req = make_rating_product_request();
    let result =
        rating_product::submit_rating(&order_repo, &rating_repo, order_id, titipers_id, req).await;

    assert!(matches!(result, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn submit_rating_product_gagal_db_error_saat_create() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingProductRepository::new();

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

    let req = make_rating_product_request();
    let result =
        rating_product::submit_rating(&order_repo, &rating_repo, order_id, titipers_id, req).await;

    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════════════════
// rating_product::get_rating
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn get_rating_product_sukses_sebagai_titipers() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingProductRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let rating = make_rating_product(order_id, titipers_id);

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    rating_repo
        .expect_find_by_order_id()
        .returning(move |_| Ok(Some(rating.clone())));

    let result = rating_product::get_rating(&order_repo, &rating_repo, order_id, titipers_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn get_rating_product_sukses_sebagai_jastiper() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingProductRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);
    let rating = make_rating_product(order_id, titipers_id);

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    rating_repo
        .expect_find_by_order_id()
        .returning(move |_| Ok(Some(rating.clone())));

    let result = rating_product::get_rating(&order_repo, &rating_repo, order_id, jastiper_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn get_rating_product_gagal_order_tidak_ditemukan() {
    let mut order_repo = MockOrderRepository::new();
    let rating_repo = MockRatingProductRepository::new();

    order_repo.expect_find_by_id().returning(|_| Ok(None));

    let result =
        rating_product::get_rating(&order_repo, &rating_repo, Uuid::new_v4(), Uuid::new_v4()).await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn get_rating_product_gagal_bukan_pemilik_order() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let rating_repo = MockRatingProductRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let result = rating_product::get_rating(&order_repo, &rating_repo, order_id, orang_lain).await;

    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn get_rating_product_gagal_rating_belum_ada() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut order_repo = MockOrderRepository::new();
    let mut rating_repo = MockRatingProductRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);

    order_repo
        .expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    rating_repo
        .expect_find_by_order_id()
        .returning(|_| Ok(None));

    let result = rating_product::get_rating(&order_repo, &rating_repo, order_id, titipers_id).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}
