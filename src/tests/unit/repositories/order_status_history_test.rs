#[cfg(test)]
mod tests {
    use serde_json::json;
    use sqlx::PgPool;
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::models::order::{CreateOrderRequest, PriceBreakdown, ShippingAddress};
    use crate::models::order_state::OrderStatus;
    use crate::models::role::Role;
    use crate::repositories::adapters::order_adapt::PgOrderRepository;
    use crate::repositories::adapters::order_status_history_adapt::PgOrderStatusHistoryRepository;
    use crate::repositories::order_repository::OrderRepository;
    use crate::repositories::order_status_history_repository::OrderStatusHistoryRepository;

    fn build_repos(pool: PgPool) -> (PgOrderRepository, Arc<PgOrderStatusHistoryRepository>) {
        let history_repo = Arc::new(PgOrderStatusHistoryRepository::new(pool.clone()));
        let order_repo = PgOrderRepository::new(pool, history_repo.clone());
        (order_repo, history_repo)
    }

    async fn create_dummy_order(
        order_repo: &PgOrderRepository,
    ) -> (Uuid, crate::models::order::Order) {
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

        let created = order_repo
            .create(
                titipers_id,
                jastiper_id,
                req,
                snapshot,
                PriceBreakdown {
                    unit_price: 10_000,
                    service_fee: 1_000,
                    total_price: 11_000,
                },
            )
            .await
            .expect("Gagal membuat dummy order");

        (titipers_id, created)
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_insert_berhasil(pool: PgPool) {
        let (order_repo, history_repo) = build_repos(pool);
        let (titipers_id, order) = create_dummy_order(&order_repo).await;

        let result = history_repo
            .insert_status_history(
                order.order_id,
                &OrderStatus::Paid,
                &titipers_id.to_string(),
                &Role::Titipers,
                Some("Pembayaran berhasil"),
            )
            .await;

        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_insert_tanpa_notes(pool: PgPool) {
        let (order_repo, history_repo) = build_repos(pool);
        let (titipers_id, order) = create_dummy_order(&order_repo).await;

        let result = history_repo
            .insert_status_history(
                order.order_id,
                &OrderStatus::Paid,
                &titipers_id.to_string(),
                &Role::Titipers,
                None,
            )
            .await;

        assert!(result.is_ok());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_history_urutan_asc(pool: PgPool) {
        let (order_repo, history_repo) = build_repos(pool);
        let (titipers_id, order) = create_dummy_order(&order_repo).await;

        history_repo
            .insert_status_history(
                order.order_id,
                &OrderStatus::Paid,
                &titipers_id.to_string(),
                &Role::Titipers,
                Some("Lunas"),
            )
            .await
            .unwrap();

        history_repo
            .insert_status_history(
                order.order_id,
                &OrderStatus::Completed,
                &titipers_id.to_string(),
                &Role::Titipers,
                Some("Selesai"),
            )
            .await
            .unwrap();

        let history = history_repo
            .get_status_history(order.order_id)
            .await
            .expect("Query gagal");

        // 1 dari create (Pending) + 2 insert manual = 3
        assert_eq!(history.len(), 3);

        for i in 1..history.len() {
            assert!(
                history[i].timestamp >= history[i - 1].timestamp,
                "History harus urut ASC berdasarkan timestamp"
            );
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_history_order_tidak_ada(pool: PgPool) {
        let (_, history_repo) = build_repos(pool);

        let history = history_repo
            .get_status_history(Uuid::new_v4())
            .await
            .expect("Query gagal");

        assert!(history.is_empty());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_history_tidak_tercampur_antar_order(pool: PgPool) {
        let (order_repo, history_repo) = build_repos(pool);
        let (titipers_id, order_a) = create_dummy_order(&order_repo).await;
        let (_, order_b) = create_dummy_order(&order_repo).await;

        history_repo
            .insert_status_history(
                order_b.order_id,
                &OrderStatus::Paid,
                &titipers_id.to_string(),
                &Role::Titipers,
                None,
            )
            .await
            .unwrap();

        let history_a = history_repo
            .get_status_history(order_a.order_id)
            .await
            .expect("Query gagal");

        assert!(
            history_a.iter().all(|h| h.order_id == order_a.order_id),
            "History order_a tidak boleh mengandung record order_b"
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_history_changed_by_dan_actor_role_tersimpan(pool: PgPool) {
        let (order_repo, history_repo) = build_repos(pool);
        let (_, order) = create_dummy_order(&order_repo).await;

        history_repo
            .insert_status_history(
                order.order_id,
                &OrderStatus::Paid,
                "admin-user-123",
                &Role::Admin,
                Some("Di-approve admin"),
            )
            .await
            .unwrap();

        let history = history_repo
            .get_status_history(order.order_id)
            .await
            .unwrap();

        let paid_entry = history
            .iter()
            .find(|h| h.status == OrderStatus::Paid.to_string().parse().unwrap());
        assert!(paid_entry.is_some());

        let entry = paid_entry.unwrap();
        assert_eq!(entry.changed_by, "admin-user-123");
        assert_eq!(entry.actor_role, Role::Admin);
        assert_eq!(entry.notes, Some("Di-approve admin".to_string()));
    }
}
