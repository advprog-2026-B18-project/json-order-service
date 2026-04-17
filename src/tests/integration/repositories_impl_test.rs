use serde_json::json;
use uuid::Uuid;

use crate::models::filter_pagination::{OrderFilter, PaginationParams};
use crate::models::order::{CreateOrderRequest, PriceBreakdown, UpdateOrderParams};
use crate::models::order_state::OrderStatus;
use crate::models::rating_jastiper::CreateRatingJastiperRequest;
use crate::models::rating_product::CreateRatingProductRequest;
use crate::models::role::Role;
use crate::models::shipping_address::ShippingAddress;
use crate::ports::order_repository::OrderRepository;
use crate::ports::order_status_history_repository::OrderStatusHistoryRepository;
use crate::ports::rating_jastiper_repository::RatingJastiperRepository;
use crate::ports::rating_product_repository::RatingProductRepository;
use crate::repositories::order_impl::PgOrderRepository;
use crate::repositories::order_status_history_impl::PgOrderStatusHistoryRepository;
use crate::repositories::rating_jastiper_impl::PgRatingJastiperRepository;
use crate::repositories::rating_product_impl::PgRatingProductRepository;

fn make_create_request(product_id: Uuid) -> CreateOrderRequest {
    CreateOrderRequest {
        product_id,
        quantity: 1,
        shipping_address: ShippingAddress {
            recipient_name: "Budi Santoso".to_string(),
            phone_number: "081234567890".to_string(),
            street: "Jl. Mawar No. 1".to_string(),
            kelurahan: "Menteng".to_string(),
            kecamatan: "Menteng".to_string(),
            city: "Jakarta Pusat".to_string(),
            province: "DKI Jakarta".to_string(),
            postal_code: "10310".to_string(),
            notes: None,
        },
        note_to_jastiper: None,
    }
}

fn make_price() -> PriceBreakdown {
    PriceBreakdown {
        unit_price: 10_000,
        service_fee: 1_000,
        total_price: 11_000,
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn pg_order_find_by_id_tidak_ditemukan(pool: sqlx::PgPool) {
    let repo = PgOrderRepository::new(pool);
    let result = repo.find_by_id(Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn pg_order_create_dan_find_by_id(pool: sqlx::PgPool) {
    let repo = PgOrderRepository::new(pool);
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let order = repo
        .create(
            titipers_id,
            jastiper_id,
            make_create_request(Uuid::new_v4()),
            json!({}),
            make_price(),
        )
        .await
        .unwrap();

    assert_eq!(order.titipers_id, titipers_id);
    assert_eq!(order.status, OrderStatus::Pending);

    let found = repo.find_by_id(order.order_id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().order_id, order.order_id);
}

#[sqlx::test(migrations = "./migrations")]
async fn pg_order_find_all_dengan_filter(pool: sqlx::PgPool) {
    use chrono::Utc;

    let repo = PgOrderRepository::new(pool);
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    for _ in 0..2 {
        repo.create(
            titipers_id,
            jastiper_id,
            make_create_request(Uuid::new_v4()),
            json!({}),
            make_price(),
        )
        .await
        .unwrap();
    }

    repo.create(
        Uuid::new_v4(),
        jastiper_id,
        make_create_request(Uuid::new_v4()),
        json!({}),
        make_price(),
    )
    .await
    .unwrap();

    let filter = OrderFilter {
        titipers_id: Some(titipers_id),
        jastiper_id: None,
        product_id: None,
        status: None,
        date_from: Utc::now() - chrono::Duration::hours(1),
        date_to: Utc::now() + chrono::Duration::hours(1),
    };

    let pagination = PaginationParams {
        page: Some(1),
        limit: Some(10),
        sort_by: None,
        order: None,
    };

    let (orders, total) = repo.find_all(Some(&filter), &pagination).await.unwrap();
    assert_eq!(orders.len(), 2);
    assert_eq!(total, 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn pg_order_update_status(pool: sqlx::PgPool) {
    let repo = PgOrderRepository::new(pool);
    let titipers_id = Uuid::new_v4();

    let order = repo
        .create(
            titipers_id,
            Uuid::new_v4(),
            make_create_request(Uuid::new_v4()),
            json!({}),
            make_price(),
        )
        .await
        .unwrap();

    let updated = repo
        .update(
            order.order_id,
            &OrderStatus::Paid,
            UpdateOrderParams {
                tracking_number: None,
                courier: None,
                cancellation_reason: None,
                notes: Some("Bayar sukses"),
                changed_by: &titipers_id.to_string(),
                actor_role: &Role::Titipers,
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.status, OrderStatus::Paid);
}

#[sqlx::test(migrations = "./migrations")]
async fn pg_order_status_history_insert_dan_get(pool: sqlx::PgPool) {
    let order_repo = PgOrderRepository::new(pool.clone());
    let order = order_repo
        .create(
            Uuid::new_v4(),
            Uuid::new_v4(),
            make_create_request(Uuid::new_v4()),
            json!({}),
            make_price(),
        )
        .await
        .unwrap();

    let history_repo = PgOrderStatusHistoryRepository::new(pool);

    history_repo
        .insert_status_history(
            order.order_id,
            &OrderStatus::Paid,
            "system",
            &Role::System,
            Some("Pembayaran dikonfirmasi"),
        )
        .await
        .unwrap();

    let history = history_repo
        .get_status_history(order.order_id)
        .await
        .unwrap();

    assert!(history.len() >= 2);
    let statuses: Vec<_> = history.iter().map(|h| &h.status).collect();
    assert!(
        statuses
            .iter()
            .any(|s| s.to_string() == OrderStatus::Paid.to_string())
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn pg_order_status_history_kosong_untuk_order_tidak_ada(pool: sqlx::PgPool) {
    let repo = PgOrderStatusHistoryRepository::new(pool);
    let result = repo.get_status_history(Uuid::new_v4()).await.unwrap();
    assert!(result.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn pg_rating_product_find_by_id_tidak_ditemukan(pool: sqlx::PgPool) {
    let repo = PgRatingProductRepository::new(pool);
    let result = repo.find_by_id(Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn pg_rating_product_find_by_order_id_tidak_ditemukan(pool: sqlx::PgPool) {
    let repo = PgRatingProductRepository::new(pool);
    let result = repo.find_by_order_id(Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn pg_rating_product_create_dan_find(pool: sqlx::PgPool) {
    let order_repo = PgOrderRepository::new(pool.clone());
    let order = order_repo
        .create(
            Uuid::new_v4(),
            Uuid::new_v4(),
            make_create_request(Uuid::new_v4()),
            json!({}),
            make_price(),
        )
        .await
        .unwrap();

    let repo = PgRatingProductRepository::new(pool);
    let titipers_id = order.titipers_id;

    let req = CreateRatingProductRequest {
        product_rating: 4.5,
        product_review: Some("Produk sangat bagus".to_string()),
        product_images: Some(vec!["https://img.example.com/1.jpg".to_string()]),
    };

    let rating = repo
        .create(order.order_id, titipers_id, &req)
        .await
        .unwrap();

    assert_eq!(rating.order_id, order.order_id);
    assert_eq!(rating.titipers_id, titipers_id);

    let found_by_id = repo.find_by_id(rating.rating_product_id).await.unwrap();
    assert!(found_by_id.is_some());

    let found_by_order = repo.find_by_order_id(order.order_id).await.unwrap();
    assert!(found_by_order.is_some());
    assert_eq!(
        found_by_order.unwrap().rating_product_id,
        rating.rating_product_id
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn pg_rating_jastiper_find_by_id_tidak_ditemukan(pool: sqlx::PgPool) {
    let repo = PgRatingJastiperRepository::new(pool);
    let result = repo.find_by_id(Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn pg_rating_jastiper_find_by_order_id_tidak_ditemukan(pool: sqlx::PgPool) {
    let repo = PgRatingJastiperRepository::new(pool);
    let result = repo.find_by_order_id(Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn pg_rating_jastiper_create_dan_find(pool: sqlx::PgPool) {
    let order_repo = PgOrderRepository::new(pool.clone());
    let order = order_repo
        .create(
            Uuid::new_v4(),
            Uuid::new_v4(),
            make_create_request(Uuid::new_v4()),
            json!({}),
            make_price(),
        )
        .await
        .unwrap();

    let repo = PgRatingJastiperRepository::new(pool);
    let titipers_id = order.titipers_id;

    let req = CreateRatingJastiperRequest {
        jastiper_rating: 5.0,
        jastiper_review: Some("Jastiper ramah dan cepat".to_string()),
    };

    let rating = repo
        .create(order.order_id, titipers_id, &req)
        .await
        .unwrap();

    assert_eq!(rating.order_id, order.order_id);
    assert_eq!(rating.titipers_id, titipers_id);

    let found_by_id = repo.find_by_id(rating.rating_jastiper_id).await.unwrap();
    assert!(found_by_id.is_some());

    let found_by_order = repo.find_by_order_id(order.order_id).await.unwrap();
    assert!(found_by_order.is_some());
    assert_eq!(
        found_by_order.unwrap().rating_jastiper_id,
        rating.rating_jastiper_id
    );
}
