#[cfg(test)]
mod tests {
    use serde_json::json;
    use sqlx::PgPool;
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::models::filter_pagination::{OrderFilter, PaginationParams};
    use crate::models::order::{
        CreateOrderRequest, PriceBreakdown, ShippingAddress, UpdateOrderParams,
    };
    use crate::models::order_state::OrderStatus;
    use crate::models::role::Role;
    use crate::repositories::adapters::order_adapt::PgOrderRepository;
    use crate::repositories::adapters::order_status_history_adapt::PgOrderStatusHistoryRepository;
    use crate::repositories::order_repository::OrderRepository;

    fn build_order_repo(pool: PgPool) -> PgOrderRepository {
        let history_repo = Arc::new(PgOrderStatusHistoryRepository::new(pool.clone()));
        PgOrderRepository::new(pool, history_repo)
    }

    async fn create_dummy_order(
        repo: &PgOrderRepository,
    ) -> (Uuid, Uuid, crate::models::order::Order) {
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

        let created = repo
            .create(
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
        let repo = build_order_repo(pool);
        let (titipers_id, _, created) = create_dummy_order(&repo).await;

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
        let repo = build_order_repo(pool);

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

        let created = repo
            .create(
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
    async fn test_create_otomatis_insert_status_history_pending(pool: PgPool) {
        let history_repo = Arc::new(PgOrderStatusHistoryRepository::new(pool.clone()));
        let repo = PgOrderRepository::new(pool.clone(), history_repo.clone());

        let (titipers_id, _, created) = create_dummy_order(&repo).await;

        use crate::repositories::order_status_history_repository::OrderStatusHistoryRepository;
        let history = history_repo
            .get_status_history(created.order_id)
            .await
            .expect("Query gagal");

        assert!(!history.is_empty());
        assert_eq!(
            history[0].status,
            OrderStatus::Pending.to_string().parse().unwrap()
        );
        assert_eq!(history[0].changed_by, titipers_id.to_string());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_by_id_ditemukan(pool: PgPool) {
        let repo = build_order_repo(pool);
        let (_, _, created) = create_dummy_order(&repo).await;

        let found = repo
            .find_by_id(created.order_id)
            .await
            .expect("Query gagal");

        assert!(found.is_some());
        assert_eq!(found.unwrap().order_id, created.order_id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_by_id_tidak_ditemukan(pool: PgPool) {
        let repo = build_order_repo(pool);

        let found = repo.find_by_id(Uuid::new_v4()).await.expect("Query gagal");

        assert!(found.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_all_tanpa_filter(pool: PgPool) {
        let repo = build_order_repo(pool);
        create_dummy_order(&repo).await;
        create_dummy_order(&repo).await;

        let pagination = PaginationParams {
            page: Some(1),
            limit: Some(10),
            sort_by: None,
            order: None,
        };

        let (orders, total) = repo.find_all(None, &pagination).await.expect("Query gagal");

        assert_eq!(orders.len(), 2);
        assert_eq!(total, 2);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_all_filter_titipers_id(pool: PgPool) {
        let repo = build_order_repo(pool);
        let (titipers_id, _, _) = create_dummy_order(&repo).await;
        create_dummy_order(&repo).await;

        let filter = OrderFilter {
            titipers_id: Some(titipers_id),
            jastiper_id: None,
            product_id: None,
            status: None,
            date_from: None,
            date_to: None,
        };

        let pagination = PaginationParams {
            page: Some(1),
            limit: Some(10),
            sort_by: None,
            order: None,
        };

        let (orders, total) = repo
            .find_all(Some(&filter), &pagination)
            .await
            .expect("Query gagal");

        assert_eq!(total, 1);
        assert_eq!(orders[0].titipers_id, titipers_id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_all_filter_status_pending(pool: PgPool) {
        let repo = build_order_repo(pool);
        create_dummy_order(&repo).await;

        let filter = OrderFilter {
            titipers_id: None,
            jastiper_id: None,
            product_id: None,
            status: Some(OrderStatus::Pending),
            date_from: None,
            date_to: None,
        };

        let pagination = PaginationParams {
            page: Some(1),
            limit: Some(10),
            sort_by: None,
            order: None,
        };

        let (orders, _) = repo
            .find_all(Some(&filter), &pagination)
            .await
            .expect("Query gagal");

        assert!(orders.iter().all(|o| o.status == OrderStatus::Pending));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_find_all_pagination(pool: PgPool) {
        let repo = build_order_repo(pool);
        for _ in 0..5 {
            create_dummy_order(&repo).await;
        }

        let pagination = PaginationParams {
            page: Some(1),
            limit: Some(3),
            sort_by: None,
            order: None,
        };

        let (orders, total) = repo.find_all(None, &pagination).await.expect("Query gagal");

        assert_eq!(total, 5);
        assert_eq!(orders.len(), 3);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_update_status(pool: PgPool) {
        let repo = build_order_repo(pool);
        let (titipers_id, _, created) = create_dummy_order(&repo).await;

        let updated = repo
            .update(
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
        let repo = build_order_repo(pool);
        let (_, jastiper_id, created) = create_dummy_order(&repo).await;

        let updated = repo
            .update(
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
        let repo = build_order_repo(pool);
        let (titipers_id, _, created) = create_dummy_order(&repo).await;

        let updated = repo
            .update(
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

    #[sqlx::test(migrations = "./migrations")]
    async fn test_update_otomatis_insert_status_history(pool: PgPool) {
        let history_repo = Arc::new(PgOrderStatusHistoryRepository::new(pool.clone()));
        let repo = PgOrderRepository::new(pool.clone(), history_repo.clone());

        let (titipers_id, _, created) = create_dummy_order(&repo).await;

        repo.update(
            created.order_id,
            &OrderStatus::Paid,
            UpdateOrderParams {
                changed_by: &titipers_id.to_string(),
                actor_role: &Role::Titipers,
                notes: Some("Lunas"),
                tracking_number: None,
                courier: None,
                cancellation_reason: None,
            },
        )
        .await
        .expect("Update gagal");

        use crate::repositories::order_status_history_repository::OrderStatusHistoryRepository;
        let history = history_repo
            .get_status_history(created.order_id)
            .await
            .expect("Query gagal");

        // history[0] = Pending (saat create), history[1] = Paid (saat update)
        assert_eq!(history.len(), 2);
        assert_eq!(
            history[1].status,
            OrderStatus::Paid.to_string().parse().unwrap()
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_delete_berhasil(pool: PgPool) {
        let repo = build_order_repo(pool);
        let (_, _, created) = create_dummy_order(&repo).await;

        repo.delete(created.order_id).await.expect("Delete gagal");

        let found = repo
            .find_by_id(created.order_id)
            .await
            .expect("Query gagal");

        assert!(found.is_none());
    }
}
