#[cfg(test)]
mod tests {
    use sqlx::PgPool;
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::models::order::{CreateOrderRequest, PriceBreakdown, ShippingAddress};
    use crate::models::rating_jastiper::CreateRatingJastiperRequest;
    use crate::repositories::adapters::order_adapt::PgOrderRepository;
    use crate::repositories::adapters::order_status_history_adapt::PgOrderStatusHistoryRepository;
    use crate::repositories::adapters::rating_jastiper_adapt::PgRatingJastiperRepository;
    use crate::repositories::order_repository::OrderRepository;
    use crate::repositories::rating_jastiper_repository::RatingJastiperRepository;

    fn build_rating_jastiper_repo(pool: PgPool) -> (PgOrderRepository, PgRatingJastiperRepository) {
        let history_repo = Arc::new(PgOrderStatusHistoryRepository::new(pool.clone()));
        let order_repo = PgOrderRepository::new(pool.clone(), history_repo);
        let rating_repo = PgRatingJastiperRepository::new(pool);
        (order_repo, rating_repo)
    }

    async fn create_dummy_order(
        order_repo: &PgOrderRepository,
    ) -> (Uuid, Uuid, crate::models::order::Order) {
        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();

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
        };

        let created = order_repo
            .create(
                titipers_id,
                jastiper_id,
                req,
                serde_json::json!({"name": "Produk Test"}),
                PriceBreakdown {
                    unit_price: 30_000,
                    service_fee: 3_000,
                    total_price: 33_000,
                },
            )
            .await
            .expect("Gagal membuat dummy order");

        (titipers_id, jastiper_id, created)
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_berhasil(pool: PgPool) {
        let (order_repo, rating_repo) = build_rating_jastiper_repo(pool);
        let (titipers_id, _, order) = create_dummy_order(&order_repo).await;

        let req = CreateRatingJastiperRequest {
            jastiper_rating: 5f64,
            jastiper_review: Some("Jastiper ramah dan cepat!".to_string()),
        };

        let rating = rating_repo
            .create(order.order_id, titipers_id, &req)
            .await
            .expect("Gagal create rating jastiper");

        assert_eq!(rating.order_id, order.order_id);
        assert_eq!(rating.titipers_id, titipers_id);
        assert_eq!(rating.jastiper_rating, 5f64);
        assert_eq!(
            rating.jastiper_review,
            Some("Jastiper ramah dan cepat!".to_string())
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_tanpa_review(pool: PgPool) {
        let (order_repo, rating_repo) = build_rating_jastiper_repo(pool);
        let (titipers_id, _, order) = create_dummy_order(&order_repo).await;

        let req = CreateRatingJastiperRequest {
            jastiper_rating: 4f64,
            jastiper_review: None,
        };

        let rating = rating_repo
            .create(order.order_id, titipers_id, &req)
            .await
            .expect("Gagal create rating jastiper tanpa review");

        assert_eq!(rating.jastiper_rating, 4f64);
        assert_eq!(rating.jastiper_review, Some("".to_string()));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_rating_minimum_dan_maksimum(pool: PgPool) {
        let (order_repo, rating_repo) = build_rating_jastiper_repo(pool);
        let (titipers_id_a, _, order_a) = create_dummy_order(&order_repo).await;
        let (titipers_id_b, _, order_b) = create_dummy_order(&order_repo).await;

        let rating_min = rating_repo
            .create(
                order_a.order_id,
                titipers_id_a,
                &CreateRatingJastiperRequest {
                    jastiper_rating: 1f64,
                    jastiper_review: None,
                },
            )
            .await
            .unwrap();

        let rating_max = rating_repo
            .create(
                order_b.order_id,
                titipers_id_b,
                &CreateRatingJastiperRequest {
                    jastiper_rating: 5f64,
                    jastiper_review: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(rating_min.jastiper_rating, 1f64);
        assert_eq!(rating_max.jastiper_rating, 5f64);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_created_at_terisi(pool: PgPool) {
        let (order_repo, rating_repo) = build_rating_jastiper_repo(pool);
        let (titipers_id, _, order) = create_dummy_order(&order_repo).await;

        let rating = rating_repo
            .create(
                order.order_id,
                titipers_id,
                &CreateRatingJastiperRequest {
                    jastiper_rating: 3f64,
                    jastiper_review: None,
                },
            )
            .await
            .unwrap();

        let diff = chrono::Utc::now() - rating.created_at;
        assert!(
            diff.num_seconds() < 5,
            "created_at seharusnya baru saja di-set"
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_by_id_ditemukan(pool: PgPool) {
        let (order_repo, rating_repo) = build_rating_jastiper_repo(pool);
        let (titipers_id, _, order) = create_dummy_order(&order_repo).await;

        let created = rating_repo
            .create(
                order.order_id,
                titipers_id,
                &CreateRatingJastiperRequest {
                    jastiper_rating: 3f64,
                    jastiper_review: Some("Lumayan".to_string()),
                },
            )
            .await
            .unwrap();

        let found = rating_repo
            .find_by_id(created.rating_jastiper_id)
            .await
            .expect("Query gagal");

        assert!(found.is_some());
        assert_eq!(
            found.unwrap().rating_jastiper_id,
            created.rating_jastiper_id
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_by_id_tidak_ditemukan(pool: PgPool) {
        let (_, rating_repo) = build_rating_jastiper_repo(pool);

        let found = rating_repo
            .find_by_id(Uuid::new_v4())
            .await
            .expect("Query gagal");

        assert!(found.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_by_order_id_ditemukan(pool: PgPool) {
        let (order_repo, rating_repo) = build_rating_jastiper_repo(pool);
        let (titipers_id, _, order) = create_dummy_order(&order_repo).await;

        rating_repo
            .create(
                order.order_id,
                titipers_id,
                &CreateRatingJastiperRequest {
                    jastiper_rating: 5f64,
                    jastiper_review: None,
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
        let (order_repo, rating_repo) = build_rating_jastiper_repo(pool);
        let (_, _, order) = create_dummy_order(&order_repo).await;

        let found = rating_repo
            .find_by_order_id(order.order_id)
            .await
            .expect("Query gagal");

        assert!(found.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_by_order_id_tidak_tercampur_antar_order(pool: PgPool) {
        let (order_repo, rating_repo) = build_rating_jastiper_repo(pool);
        let (titipers_id, _, order_a) = create_dummy_order(&order_repo).await;
        let (_, _, order_b) = create_dummy_order(&order_repo).await;

        rating_repo
            .create(
                order_a.order_id,
                titipers_id,
                &CreateRatingJastiperRequest {
                    jastiper_rating: 5f64,
                    jastiper_review: None,
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

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_by_order_id_data_lengkap_sesuai(pool: PgPool) {
        let (order_repo, rating_repo) = build_rating_jastiper_repo(pool);
        let (titipers_id, _, order) = create_dummy_order(&order_repo).await;

        rating_repo
            .create(
                order.order_id,
                titipers_id,
                &CreateRatingJastiperRequest {
                    jastiper_rating: 2f64,
                    jastiper_review: Some("Pengiriman lambat".to_string()),
                },
            )
            .await
            .unwrap();

        let found = rating_repo
            .find_by_order_id(order.order_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(found.titipers_id, titipers_id);
        assert_eq!(found.jastiper_rating, 2f64);
        assert_eq!(found.jastiper_review, Some("Pengiriman lambat".to_string()));
    }
}
