#[cfg(test)]
mod tests {
    use serde_json::Value;
    use sqlx::PgPool;
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::models::filter_pagination::PaginationParams;
    use crate::models::order::{CreateOrderRequest, PriceBreakdown, ShippingAddress};
    use crate::models::rating_product::CreateRatingProductRequest;
    use crate::repositories::adapters::order_adapt::PgOrderRepository;
    use crate::repositories::adapters::order_status_history_adapt::PgOrderStatusHistoryRepository;
    use crate::repositories::adapters::rating_product_adapt::PgRatingProductRepository;
    use crate::repositories::order_repository::OrderRepository;
    use crate::repositories::rating_product_repository::RatingProductRepository;

    fn build_rating_product_repo(pool: PgPool) -> (PgOrderRepository, PgRatingProductRepository) {
        let history_repo = Arc::new(PgOrderStatusHistoryRepository::new(pool.clone()));
        let order_repo = PgOrderRepository::new(pool.clone(), history_repo);
        let rating_repo = PgRatingProductRepository::new(pool);
        (order_repo, rating_repo)
    }

    async fn create_dummy_order(
        order_repo: &PgOrderRepository,
    ) -> (Uuid, crate::models::order::Order) {
        let titipers_id = Uuid::new_v4();

        let req = CreateOrderRequest {
            product_id: Uuid::new_v4(),
            quantity: 1,
            shipping_address: ShippingAddress {
                recipient_name: "".to_string(),
                phone_number: "".to_string(),
                street: "Jl. Margonda Raya No. 1".to_string(),
                kelurahan: "".to_string(),
                kecamatan: "".to_string(),
                city: "Depok".to_string(),
                province: "Jawa Barat".to_string(),
                postal_code: "16424".to_string(),
                notes: None,
            },
            note_to_jastiper: None,
            idempotency_key: None,
        };

        let created = order_repo
            .create(
                Uuid::new_v4(),
                titipers_id,
                Uuid::new_v4(),
                req,
                serde_json::json!({"name": "Produk Test"}),
                PriceBreakdown {
                    unit_price: 20_000,
                    service_fee: 2_000,
                    total_price: 22_000,
                },
            )
            .await
            .expect("Gagal membuat dummy order");

        (titipers_id, created)
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_berhasil(pool: PgPool) {
        let (order_repo, rating_repo) = build_rating_product_repo(pool);
        let (titipers_id, order) = create_dummy_order(&order_repo).await;

        let req = CreateRatingProductRequest {
            product_rating: 5f64,
            product_review: Some("Produknya sangat bagus!".to_string()),
            product_images: Some(vec![
                "https://cdn.example.com/img1.jpg".to_string(),
                "https://cdn.example.com/img2.jpg".to_string(),
            ]),
        };

        let rating = rating_repo
            .create(order.order_id, titipers_id, &req)
            .await
            .expect("Gagal create rating product");

        assert_eq!(rating.order_id, order.order_id);
        assert_eq!(rating.titipers_id, titipers_id);
        assert_eq!(rating.product_rating, 5f64);
        assert_eq!(
            rating.product_review,
            Some("Produknya sangat bagus!".to_string())
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_images_tersimpan_dengan_benar(pool: PgPool) {
        let (order_repo, rating_repo) = build_rating_product_repo(pool);
        let (titipers_id, order) = create_dummy_order(&order_repo).await;

        let images = vec![
            "https://cdn.example.com/a.jpg".to_string(),
            "https://cdn.example.com/b.jpg".to_string(),
        ];

        let req = CreateRatingProductRequest {
            product_rating: 4f64,
            product_review: None,
            product_images: Some(images.clone()),
        };

        let rating = rating_repo
            .create(order.order_id, titipers_id, &req)
            .await
            .unwrap();

        let stored: Vec<String> =
            serde_json::from_value(Value::from(rating.product_images)).expect("Gagal parse images");
        assert_eq!(stored, images);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_tanpa_review_dan_images(pool: PgPool) {
        let (order_repo, rating_repo) = build_rating_product_repo(pool);
        let (titipers_id, order) = create_dummy_order(&order_repo).await;

        let req = CreateRatingProductRequest {
            product_rating: 3f64,
            product_review: None,
            product_images: None,
        };

        let rating = rating_repo
            .create(order.order_id, titipers_id, &req)
            .await
            .expect("Gagal create rating product tanpa review");

        assert_eq!(rating.product_rating, 3f64);
        assert_eq!(rating.product_review, Some("".to_string()));
        assert_eq!(rating.product_images, Vec::<String>::new());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_rating_minimum_dan_maksimum(pool: PgPool) {
        let (order_repo, rating_repo) = build_rating_product_repo(pool);
        let (titipers_id_a, order_a) = create_dummy_order(&order_repo).await;
        let (titipers_id_b, order_b) = create_dummy_order(&order_repo).await;

        let rating_min = rating_repo
            .create(
                order_a.order_id,
                titipers_id_a,
                &CreateRatingProductRequest {
                    product_rating: 1f64,
                    product_review: None,
                    product_images: None,
                },
            )
            .await
            .unwrap();

        let rating_max = rating_repo
            .create(
                order_b.order_id,
                titipers_id_b,
                &CreateRatingProductRequest {
                    product_rating: 5f64,
                    product_review: None,
                    product_images: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(rating_min.product_rating, 1f64);
        assert_eq!(rating_max.product_rating, 5f64);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_by_id_ditemukan(pool: PgPool) {
        let (order_repo, rating_repo) = build_rating_product_repo(pool);
        let (titipers_id, order) = create_dummy_order(&order_repo).await;

        let created = rating_repo
            .create(
                order.order_id,
                titipers_id,
                &CreateRatingProductRequest {
                    product_rating: 4f64,
                    product_review: Some("Oke lah".to_string()),
                    product_images: None,
                },
            )
            .await
            .unwrap();

        let found = rating_repo
            .find_by_id(created.rating_product_id)
            .await
            .expect("Query gagal");

        assert!(found.is_some());
        assert_eq!(found.unwrap().rating_product_id, created.rating_product_id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_by_id_tidak_ditemukan(pool: PgPool) {
        let (_, rating_repo) = build_rating_product_repo(pool);

        let found = rating_repo
            .find_by_id(Uuid::new_v4())
            .await
            .expect("Query gagal");

        assert!(found.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_by_order_id_ditemukan(pool: PgPool) {
        let (order_repo, rating_repo) = build_rating_product_repo(pool);
        let (titipers_id, order) = create_dummy_order(&order_repo).await;

        rating_repo
            .create(
                order.order_id,
                titipers_id,
                &CreateRatingProductRequest {
                    product_rating: 5f64,
                    product_review: None,
                    product_images: None,
                },
            )
            .await
            .unwrap();

        let found = rating_repo
            .find_by_order_id(order.order_id)
            .await
            .expect("Query gagal");

        assert!(found.is_some());
        assert_eq!(found.unwrap().order_id, order.order_id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_by_order_id_belum_ada_rating(pool: PgPool) {
        let (order_repo, rating_repo) = build_rating_product_repo(pool);
        let (_, order) = create_dummy_order(&order_repo).await;

        let found = rating_repo
            .find_by_order_id(order.order_id)
            .await
            .expect("Query gagal");

        assert!(found.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_by_order_id_tidak_tercampur_antar_order(pool: PgPool) {
        let (order_repo, rating_repo) = build_rating_product_repo(pool);
        let (titipers_id, order_a) = create_dummy_order(&order_repo).await;
        let (_, order_b) = create_dummy_order(&order_repo).await;

        rating_repo
            .create(
                order_a.order_id,
                titipers_id,
                &CreateRatingProductRequest {
                    product_rating: 5f64,
                    product_review: None,
                    product_images: None,
                },
            )
            .await
            .unwrap();

        let found_b = rating_repo
            .find_by_order_id(order_b.order_id)
            .await
            .expect("Query gagal");

        assert!(
            found_b.is_none(),
            "order_b tidak boleh mendapat rating milik order_a"
        );
    }

    // === find_all_by_product_id ===

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_all_by_product_id_returns_ratings(pool: PgPool) {
        let (order_repo, rating_repo) = build_rating_product_repo(pool);
        let (titipers_id, order) = create_dummy_order(&order_repo).await;

        rating_repo
            .create(
                order.order_id,
                titipers_id,
                &CreateRatingProductRequest {
                    product_rating: 4f64,
                    product_review: Some("Produk bagus".to_string()),
                    product_images: None,
                },
            )
            .await
            .unwrap();

        let pagination = PaginationParams {
            page: Some(1),
            limit: Some(10),
            sort_by: None,
            order: None,
        };

        let (ratings, total) = rating_repo
            .find_all_by_product_id(order.product_id, &pagination)
            .await
            .expect("Query gagal");

        assert_eq!(ratings.len(), 1);
        assert_eq!(total, 1);
        assert_eq!(ratings[0].product_rating, 4f64);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_all_by_product_id_no_ratings(pool: PgPool) {
        let (_, rating_repo) = build_rating_product_repo(pool);

        let pagination = PaginationParams {
            page: Some(1),
            limit: Some(10),
            sort_by: None,
            order: None,
        };

        let (ratings, total) = rating_repo
            .find_all_by_product_id(Uuid::new_v4(), &pagination)
            .await
            .expect("Query gagal");

        assert!(ratings.is_empty());
        assert_eq!(total, 0);
    }
}
