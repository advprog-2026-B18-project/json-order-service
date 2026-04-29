#[cfg(test)]
mod tests {
    use serde_json::json;
    use sqlx::PgPool;
    use uuid::Uuid;

    use crate::models::filter_pagination::{OrderFilter, PaginationParams};
    use crate::models::order::{
        CreateOrderRequest, PriceBreakdown, ShippingAddress, UpdateOrderParams,
    };
    use crate::models::order_state::OrderStatus;
    use crate::models::role::Role;
    use crate::repositories::order;

    async fn create_dummy_order(pool: &PgPool) -> (Uuid, Uuid, order::Order) {
        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();

        let req = CreateOrderRequest {
            product_id: Uuid::new_v4(),
            quantity: 2,
            shipping_address: ShippingAddress {
                recipient_name: "Adpro".to_string(),
                phone_number: "08123456789".to_string(),
                street: "Jl. Margonda Raya No. 1".to_string(),
                kelurahan: "Ratu Jaya".to_string(),
                kecamatan: "Pancoran Mas".to_string(),
                city: "Depok".to_string(),
                province: "Jawa Barat".to_string(),
                postal_code: "16424".to_string(),
                notes: None,
            },
            note_to_jastiper: Some("Tolong dibungkus rapi".to_string()),
        };

        let snapshot = json!({
            "product_id": "3b07c9e0-8e1b-4fbb-9dd2-3e6aa79111fb",
            "name": "Sepatu Nike Air Force 1",
            "description": "Sepatu original limited edition",
            "image_url": "https://example.com/images/sepatu-nike.jpg",
            "origin_country": "Vietnam",
            "purchase_date": "2025-12-01",
            "unit_price": 850_000,
            "service_fee": 50_000
        });

        let created = order::create(
            pool,
            titipers_id,
            jastiper_id,
            req,
            snapshot,
            PriceBreakdown {
                unit_price: 25_000,
                service_fee: 2_000,
                total_price: 52_000,
            },
        )
        .await
        .expect("Gagal membuat dummy order");

        (titipers_id, jastiper_id, created)
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_berhasil(pool: PgPool) {
        let (titipers_id, _, created) = create_dummy_order(&pool).await;

        assert_eq!(created.titipers_id, titipers_id);
        assert_eq!(created.quantity, 2);
        assert_eq!(created.unit_price, 25_000);
        assert_eq!(created.service_fee, 2_000);
        assert_eq!(created.total_price, 52_000);
        assert_eq!(created.status, OrderStatus::Pending);
        assert!(created.tracking_number.is_none());
        assert!(created.completed_at.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_create_note_default_kosong(pool: PgPool) {
        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();

        let req = CreateOrderRequest {
            product_id: Uuid::new_v4(),
            quantity: 1,
            shipping_address: ShippingAddress {
                recipient_name: "Burhan".to_string(),
                phone_number: "08123456789".to_string(),
                street: "Jl. UI No. 5".to_string(),
                kelurahan: "Ratu Jaya".to_string(),
                kecamatan: "Pancoran Mas".to_string(),
                city: "Depok".to_string(),
                province: "Jawa Barat".to_string(),
                postal_code: "16425".to_string(),
                notes: None,
            },
            note_to_jastiper: None,
        };

        let created = order::create(
            &pool,
            titipers_id,
            jastiper_id,
            req,
            json!({"name": "Produk A"}),
            PriceBreakdown {
                unit_price: 10_000,
                service_fee: 1_000,
                total_price: 11_000,
            },
        )
        .await
        .expect("Gagal create order tanpa note");

        assert_eq!(created.note_to_jastiper.unwrap().as_str(), "");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_by_id_ditemukan(pool: PgPool) {
        let (_, _, created) = create_dummy_order(&pool).await;

        let found = order::find_by_id(&pool, created.order_id)
            .await
            .expect("Query gagal");

        assert!(found.is_some());
        assert_eq!(found.unwrap().order_id, created.order_id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_by_id_tidak_ditemukan(pool: PgPool) {
        let found = order::find_by_id(&pool, Uuid::new_v4())
            .await
            .expect("Query gagal");

        assert!(found.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_all_tanpa_filter(pool: PgPool) {
        create_dummy_order(&pool).await;
        create_dummy_order(&pool).await;

        let pagination = PaginationParams {
            page: Some(1),
            limit: Some(10),
            sort_by: None,
            order: None,
        };

        let (orders, total) = order::find_all(&pool, None, &pagination)
            .await
            .expect("Query gagal");

        assert_eq!(orders.len(), 2);
        assert_eq!(total, 2);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_all_filter_titipers_id(pool: PgPool) {
        let (titipers_id, _, _) = create_dummy_order(&pool).await;
        create_dummy_order(&pool).await;

        let filter = OrderFilter {
            titipers_id: Some(titipers_id),
            jastiper_id: None,
            product_id: None,
            status: None,
            date_from: chrono::DateTime::from_timestamp(0, 0).unwrap(),
            date_to: chrono::Utc::now(),
        };

        let pagination = PaginationParams {
            page: Some(1),
            limit: Some(10),
            sort_by: None,
            order: None,
        };

        let (orders, total) = order::find_all(&pool, Some(&filter), &pagination)
            .await
            .expect("Query gagal");

        assert_eq!(total, 1);
        assert_eq!(orders[0].titipers_id, titipers_id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_all_filter_status_pending(pool: PgPool) {
        create_dummy_order(&pool).await;

        let filter = OrderFilter {
            titipers_id: None,
            jastiper_id: None,
            product_id: None,
            status: Some(OrderStatus::Pending),
            date_from: chrono::DateTime::from_timestamp(0, 0).unwrap(),
            date_to: chrono::Utc::now(),
        };

        let pagination = PaginationParams {
            page: Some(1),
            limit: Some(10),
            sort_by: None,
            order: None,
        };

        let (orders, _) = order::find_all(&pool, Some(&filter), &pagination)
            .await
            .expect("Query gagal");

        assert!(orders.iter().all(|o| o.status == OrderStatus::Pending));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_all_pagination(pool: PgPool) {
        for _ in 0..5 {
            create_dummy_order(&pool).await;
        }

        let pagination = PaginationParams {
            page: Some(1),
            limit: Some(3),
            sort_by: None,
            order: None,
        };

        let (orders, total) = order::find_all(&pool, None, &pagination)
            .await
            .expect("Query gagal");

        assert_eq!(total, 5);
        assert_eq!(orders.len(), 3);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_update_status(pool: PgPool) {
        let (titipers_id, _, created) = create_dummy_order(&pool).await;

        let updated = order::update(
            &pool,
            created.order_id,
            &OrderStatus::Paid,
            UpdateOrderParams {
                changed_by: &titipers_id.to_string(),
                actor_role: &Role::Titipers,
                notes: Some("Pembayaran diterima"),
                tracking_number: None,
                courier: None,
                cancellation_reason: None,
            },
        )
        .await
        .expect("Update gagal");

        assert_eq!(updated.status, OrderStatus::Paid);
        assert_eq!(updated.order_id, created.order_id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_update_completed_set_completed_at(pool: PgPool) {
        let (_, jastiper_id, created) = create_dummy_order(&pool).await;

        let updated = order::update(
            &pool,
            created.order_id,
            &OrderStatus::Completed,
            UpdateOrderParams {
                changed_by: &jastiper_id.to_string(),
                actor_role: &Role::Jastiper,
                notes: Some("Pesanan selesai dikirim"),
                tracking_number: Some("JNE-12345"),
                courier: Some("JNE"),
                cancellation_reason: None,
            },
        )
        .await
        .expect("Update gagal");

        assert_eq!(updated.status, OrderStatus::Completed);
        assert!(updated.completed_at.is_some(), "completed_at harus terisi");
        assert_eq!(updated.tracking_number.as_deref(), Some("JNE-12345"));
        assert_eq!(updated.courier.as_deref(), Some("JNE"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_update_cancelled_dengan_alasan(pool: PgPool) {
        let (titipers_id, _, created) = create_dummy_order(&pool).await;

        let updated = order::update(
            &pool,
            created.order_id,
            &OrderStatus::Cancelled,
            UpdateOrderParams {
                changed_by: &titipers_id.to_string(),
                actor_role: &Role::Titipers,
                notes: Some("Dibatalkan oleh titipers"),
                tracking_number: None,
                courier: None,
                cancellation_reason: Some("Barang tidak tersedia"),
            },
        )
        .await
        .expect("Update gagal");

        assert_eq!(updated.status, OrderStatus::Cancelled);
    }
}
