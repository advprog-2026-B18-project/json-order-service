#[cfg(test)]
mod tests {
    use serde_json::json;
    use sqlx::PgPool;
    use uuid::Uuid;

    use crate::models::order::{CreateOrderRequest, PriceBreakdown, ShippingAddress};
    use crate::models::order_state::OrderStatus;
    use crate::models::role::Role;
    use crate::repositories::{order, order_status_history};

    async fn create_dummy_order(pool: &PgPool) -> (Uuid, order::Order) {
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

        let created = order::create(
            pool,
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
        let (titipers_id, order) = create_dummy_order(&pool).await;

        let result = order_status_history::insert_status_history(
            &pool,
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
        let (titipers_id, order) = create_dummy_order(&pool).await;

        let result = order_status_history::insert_status_history(
            &pool,
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
        let (titipers_id, order) = create_dummy_order(&pool).await;

        order_status_history::insert_status_history(
            &pool,
            order.order_id,
            &OrderStatus::Paid,
            &titipers_id.to_string(),
            &Role::Titipers,
            Some("Lunas"),
        )
        .await
        .unwrap();

        order_status_history::insert_status_history(
            &pool,
            order.order_id,
            &OrderStatus::Completed,
            &titipers_id.to_string(),
            &Role::Titipers,
            Some("Selesai"),
        )
        .await
        .unwrap();

        let history = order_status_history::get_status_history(&pool, order.order_id)
            .await
            .expect("Query gagal");

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
        let history = order_status_history::get_status_history(&pool, Uuid::new_v4())
            .await
            .expect("Query gagal");

        assert!(history.is_empty());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_history_tidak_tercampur_antar_order(pool: PgPool) {
        let (titipers_id, order_a) = create_dummy_order(&pool).await;
        let (_, order_b) = create_dummy_order(&pool).await;

        order_status_history::insert_status_history(
            &pool,
            order_b.order_id,
            &OrderStatus::Paid,
            &titipers_id.to_string(),
            &Role::Titipers,
            None,
        )
        .await
        .unwrap();

        let history_a = order_status_history::get_status_history(&pool, order_a.order_id)
            .await
            .expect("Query gagal");

        assert!(
            history_a.iter().all(|h| h.order_id == order_a.order_id),
            "History order_a tidak boleh mengandung record order_b"
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn test_get_history_changed_by_dan_actor_role_tersimpan(pool: PgPool) {
        let (_, order) = create_dummy_order(&pool).await;

        order_status_history::insert_status_history(
            &pool,
            order.order_id,
            &OrderStatus::Paid,
            "admin-user-123",
            &Role::Admin,
            Some("Di-approve admin"),
        )
        .await
        .unwrap();

        let history = order_status_history::get_status_history(&pool, order.order_id)
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
