#[cfg(test)]
mod tests {
    use sqlx::PgPool;
    use uuid::Uuid;

    use crate::models::order::{CreateOrderRequest, ShippingAddress};
    use crate::models::rating_jastiper::CreateRatingJastiperRequest;
    use crate::repositories::{order, rating_jastiper};

    async fn create_dummy_order(pool: &PgPool) -> (Uuid, Uuid, crate::models::order::Order) {
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

        let created = order::create(
            pool,
            titipers_id,
            jastiper_id,
            req,
            serde_json::json!({"name": "Produk Test"}),
            30_000,
            3_000,
            33_000,
        )
        .await
        .expect("Gagal membuat dummy order");

        (titipers_id, jastiper_id, created)
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_berhasil(pool: PgPool) {
        let (titipers_id, _, order) = create_dummy_order(&pool).await;

        let req = CreateRatingJastiperRequest {
            jastiper_rating: 5f64,
            jastiper_review: Some("Jastiper ramah dan cepat!".to_string()),
        };

        let rating = rating_jastiper::create(&pool, order.order_id, titipers_id, &req)
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
        let (titipers_id, _, order) = create_dummy_order(&pool).await;

        let req = CreateRatingJastiperRequest {
            jastiper_rating: 4f64,
            jastiper_review: None,
        };

        let rating = rating_jastiper::create(&pool, order.order_id, titipers_id, &req)
            .await
            .expect("Gagal create rating jastiper tanpa review");

        assert_eq!(rating.jastiper_rating, 4f64);
        assert_eq!(rating.jastiper_review, Some("".to_string()));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_rating_minimum_dan_maksimum(pool: PgPool) {
        let (titipers_id_a, _, order_a) = create_dummy_order(&pool).await;
        let (titipers_id_b, _, order_b) = create_dummy_order(&pool).await;

        let rating_min = rating_jastiper::create(
            &pool,
            order_a.order_id,
            titipers_id_a,
            &CreateRatingJastiperRequest {
                jastiper_rating: 1f64,
                jastiper_review: None,
            },
        )
        .await
        .unwrap();

        let rating_max = rating_jastiper::create(
            &pool,
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
        let (titipers_id, _, order) = create_dummy_order(&pool).await;

        let rating = rating_jastiper::create(
            &pool,
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
        let (titipers_id, _, order) = create_dummy_order(&pool).await;

        let created = rating_jastiper::create(
            &pool,
            order.order_id,
            titipers_id,
            &CreateRatingJastiperRequest {
                jastiper_rating: 3f64,
                jastiper_review: Some("Lumayan".to_string()),
            },
        )
        .await
        .unwrap();

        let found = rating_jastiper::find_by_id(&pool, created.rating_jastiper_id)
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
        let found = rating_jastiper::find_by_id(&pool, Uuid::new_v4())
            .await
            .expect("Query gagal");

        assert!(found.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_by_order_id_ditemukan(pool: PgPool) {
        let (titipers_id, _, order) = create_dummy_order(&pool).await;

        rating_jastiper::create(
            &pool,
            order.order_id,
            titipers_id,
            &CreateRatingJastiperRequest {
                jastiper_rating: 5f64,
                jastiper_review: None,
            },
        )
        .await
        .unwrap();

        let found = rating_jastiper::find_by_order_id(&pool, order.order_id)
            .await
            .expect("Query gagal");

        assert!(found.is_some());
        assert_eq!(found.unwrap().order_id, order.order_id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_by_order_id_belum_ada_rating(pool: PgPool) {
        let (_, _, order) = create_dummy_order(&pool).await;

        let found = rating_jastiper::find_by_order_id(&pool, order.order_id)
            .await
            .expect("Query gagal");

        assert!(found.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_by_order_id_tidak_tercampur_antar_order(pool: PgPool) {
        let (titipers_id, _, order_a) = create_dummy_order(&pool).await;
        let (_, _, order_b) = create_dummy_order(&pool).await;

        rating_jastiper::create(
            &pool,
            order_a.order_id,
            titipers_id,
            &CreateRatingJastiperRequest {
                jastiper_rating: 5f64,
                jastiper_review: None,
            },
        )
        .await
        .unwrap();

        let found_b = rating_jastiper::find_by_order_id(&pool, order_b.order_id)
            .await
            .expect("Query gagal");

        assert!(
            found_b.is_none(),
            "order_b tidak boleh mendapat rating milik order_a"
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_by_order_id_data_lengkap_sesuai(pool: PgPool) {
        let (titipers_id, _, order) = create_dummy_order(&pool).await;

        rating_jastiper::create(
            &pool,
            order.order_id,
            titipers_id,
            &CreateRatingJastiperRequest {
                jastiper_rating: 2f64,
                jastiper_review: Some("Pengiriman lambat".to_string()),
            },
        )
        .await
        .unwrap();

        let found = rating_jastiper::find_by_order_id(&pool, order.order_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(found.titipers_id, titipers_id);
        assert_eq!(found.jastiper_rating, 2f64);
        assert_eq!(found.jastiper_review, Some("Pengiriman lambat".to_string()));
    }
}
