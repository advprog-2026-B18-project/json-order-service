#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use chrono::Utc;
    use serde_json::json;
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::models::order::{Order, OrderStatus};
    use crate::repositories::order_repository::MockOrderRepository;
    use crate::repositories::order_status_history_repository::MockOrderStatusHistoryRepository;
    use crate::repositories::rating_jastiper_repository::MockRatingJastiperRepository;
    use crate::repositories::rating_product_repository::MockRatingProductRepository;
    use crate::services::auth_client::MockAuthClient;
    use crate::services::inventory_client::MockInventoryClient;
    use crate::services::wallet_client::MockWalletClient;
    use crate::state::AppState;
    use crate::tests::unit::controller::helper_test::{
        TestApp, dummy_mq_pool, json_request_internal, json_request_internal_post,
        noop_checkout_publisher, noop_idempotency_repo,
    };

    pub fn setup_service_key() {
        unsafe {
            std::env::set_var("INTERNAL_SERVICE_KEY", "valid-service-key-123");
        }
    }

    const VALID_KEY: &str = "valid-service-key-123";
    const INVALID_KEY: &str = "wrong-key";

    fn make_order(
        order_id: Uuid,
        titipers_id: Uuid,
        jastiper_id: Uuid,
        status: OrderStatus,
    ) -> Order {
        Order {
            order_id,
            titipers_id,
            jastiper_id,
            product_id: Uuid::new_v4(),
            product_snapshot: json!({}),
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expired_at: Utc::now(),
        }
    }

    fn default_state(repo: MockOrderRepository) -> AppState {
        AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(MockRatingProductRepository::new()),
            rating_jastiper_repo: Arc::new(MockRatingJastiperRepository::new()),
            auth_client: Arc::new(MockAuthClient::new()),
            checkout_publisher: Arc::new(noop_checkout_publisher()),
            mq_pool: dummy_mq_pool(),
            idempotency_repo: Arc::new(noop_idempotency_repo()),
        }
    }

    #[tokio::test]
    async fn payment_info_sukses_200() {
        setup_service_key();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(order.clone())));

        let app = TestApp::new(default_state(repo));
        let req = json_request_internal(
            "GET",
            &format!("/internal/orders/{}/payment-info", order_id),
            VALID_KEY,
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["order_id"], order_id.to_string());
        assert_eq!(body["data"]["total_price"], 11_000_i64);
        assert!(body["data"]["titipers_user_id"].is_string());
        assert!(body["data"]["jastiper_user_id"].is_string());
    }

    #[tokio::test]
    async fn payment_info_gagal_service_key_invalid_401() {
        setup_service_key();

        let order_id = Uuid::new_v4();
        let app = TestApp::new(default_state(MockOrderRepository::new()));

        let req = json_request_internal(
            "GET",
            &format!("/internal/orders/{}/payment-info", order_id),
            INVALID_KEY,
        );
        let (status, _) = app.send(req).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn payment_info_gagal_order_tidak_ditemukan_404() {
        setup_service_key();

        let order_id = Uuid::new_v4();
        let mut repo = MockOrderRepository::new();
        repo.expect_find_by_id().returning(|_| Ok(None));

        let app = TestApp::new(default_state(repo));
        let req = json_request_internal(
            "GET",
            &format!("/internal/orders/{}/payment-info", order_id),
            VALID_KEY,
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["success"], false);
    }

    #[tokio::test]
    async fn payment_info_gagal_tanpa_service_key_401() {
        setup_service_key();

        let order_id = Uuid::new_v4();
        let app = TestApp::new(default_state(MockOrderRepository::new()));

        let req = axum::http::Request::builder()
            .method("GET")
            .uri(format!("/internal/orders/{}/payment-info", order_id))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::empty())
            .unwrap();

        let (status, _) = app.send(req).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn payment_confirmed_sukses_200() {
        setup_service_key();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let pending = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
        let paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);

        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(pending.clone())));
        repo.expect_update()
            .returning(move |_, _, _| Ok(paid.clone()));

        let app = TestApp::new(default_state(repo));
        let req = json_request_internal_post(
            &format!("/internal/orders/{}/payment-confirmed", order_id),
            VALID_KEY,
            Some(json!({
                "wallet_transaction_id": Uuid::new_v4(),
                "amount_deducted": 11_000_i64,
            })),
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        assert_eq!(body["message"], "Status order diperbarui ke PAID");
        assert_eq!(body["data"]["order_id"], order_id.to_string());
    }

    #[tokio::test]
    async fn payment_confirmed_gagal_service_key_invalid_401() {
        setup_service_key();

        let order_id = Uuid::new_v4();
        let app = TestApp::new(default_state(MockOrderRepository::new()));

        let req = json_request_internal_post(
            &format!("/internal/orders/{}/payment-confirmed", order_id),
            INVALID_KEY,
            Some(json!({
                "wallet_transaction_id": Uuid::new_v4(),
                "amount_deducted": 11_000_i64,
            })),
        );
        let (status, _) = app.send(req).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn payment_confirmed_gagal_order_tidak_ditemukan_404() {
        setup_service_key();

        let order_id = Uuid::new_v4();
        let mut repo = MockOrderRepository::new();
        repo.expect_find_by_id().returning(|_| Ok(None));

        let app = TestApp::new(default_state(repo));
        let req = json_request_internal_post(
            &format!("/internal/orders/{}/payment-confirmed", order_id),
            VALID_KEY,
            Some(json!({
                "wallet_transaction_id": Uuid::new_v4(),
                "amount_deducted": 11_000_i64,
            })),
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["success"], false);
    }

    #[tokio::test]
    async fn payment_confirmed_gagal_amount_mismatch_422() {
        setup_service_key();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let pending = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(pending.clone())));

        let app = TestApp::new(default_state(repo));
        let req = json_request_internal_post(
            &format!("/internal/orders/{}/payment-confirmed", order_id),
            VALID_KEY,
            Some(json!({
                "wallet_transaction_id": Uuid::new_v4(),
                "amount_deducted": 9_999_i64,
            })),
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["success"], false);
    }

    #[tokio::test]
    async fn payment_confirmed_gagal_order_sudah_paid_409() {
        setup_service_key();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(paid.clone())));

        let app = TestApp::new(default_state(repo));
        let req = json_request_internal_post(
            &format!("/internal/orders/{}/payment-confirmed", order_id),
            VALID_KEY,
            Some(json!({
                "wallet_transaction_id": Uuid::new_v4(),
                "amount_deducted": 11_000_i64,
            })),
        );
        let (status, _) = app.send(req).await;

        assert_eq!(status, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn payment_confirmed_gagal_status_bukan_pending_409() {
        setup_service_key();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let shipped = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);
        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(shipped.clone())));

        let app = TestApp::new(default_state(repo));
        let req = json_request_internal_post(
            &format!("/internal/orders/{}/payment-confirmed", order_id),
            VALID_KEY,
            Some(json!({
                "wallet_transaction_id": Uuid::new_v4(),
                "amount_deducted": 11_000_i64,
            })),
        );
        let (status, _) = app.send(req).await;

        assert_eq!(status, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn refund_confirmed_sukses_200() {
        setup_service_key();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let refunding = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);
        let cancelled = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Cancelled);

        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(refunding.clone())));
        repo.expect_update()
            .returning(move |_, _, _| Ok(cancelled.clone()));

        let app = TestApp::new(default_state(repo));
        let req = json_request_internal_post(
            &format!("/internal/orders/{}/refund-confirmed", order_id),
            VALID_KEY,
            Some(json!({
                "success": true,
                "wallet_transaction_id": Uuid::new_v4(),
                "amount_refunded": 11_000_i64,
                "notes": null,
            })),
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        assert_eq!(body["message"], "Refund terkonfirmasi");
        assert_eq!(body["data"]["refund_confirmed"], true);
    }

    #[tokio::test]
    async fn refund_confirmed_gagal_service_key_invalid_401() {
        setup_service_key();

        let order_id = Uuid::new_v4();
        let app = TestApp::new(default_state(MockOrderRepository::new()));

        let req = json_request_internal_post(
            &format!("/internal/orders/{}/refund-confirmed", order_id),
            INVALID_KEY,
            Some(json!({
                "success": true,
                "wallet_transaction_id": Uuid::new_v4(),
                "amount_refunded": 11_000_i64,
            })),
        );
        let (status, _) = app.send(req).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn refund_confirmed_gagal_amount_mismatch_422() {
        setup_service_key();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let refunding = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);
        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(refunding.clone())));

        let app = TestApp::new(default_state(repo));
        let req = json_request_internal_post(
            &format!("/internal/orders/{}/refund-confirmed", order_id),
            VALID_KEY,
            Some(json!({
                "success": true,
                "wallet_transaction_id": Uuid::new_v4(),
                "amount_refunded": 9_999_i64,
            })),
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["success"], false);
    }

    #[tokio::test]
    async fn refund_confirmed_gagal_sudah_cancelled_409() {
        setup_service_key();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let cancelled = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Cancelled);
        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(cancelled.clone())));

        let app = TestApp::new(default_state(repo));
        let req = json_request_internal_post(
            &format!("/internal/orders/{}/refund-confirmed", order_id),
            VALID_KEY,
            Some(json!({
                "success": true,
                "wallet_transaction_id": Uuid::new_v4(),
                "amount_refunded": 11_000_i64,
            })),
        );
        let (status, _) = app.send(req).await;

        assert_eq!(status, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn refund_confirmed_gagal_status_bukan_refunding_409() {
        setup_service_key();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(paid.clone())));

        let app = TestApp::new(default_state(repo));
        let req = json_request_internal_post(
            &format!("/internal/orders/{}/refund-confirmed", order_id),
            VALID_KEY,
            Some(json!({
                "success": true,
                "wallet_transaction_id": Uuid::new_v4(),
                "amount_refunded": 11_000_i64,
            })),
        );
        let (status, _) = app.send(req).await;

        assert_eq!(status, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn refund_confirmed_gagal_success_false_ke_refund_failed_200() {
        setup_service_key();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let refunding = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);
        let refund_failed = make_order(
            order_id,
            titipers_id,
            jastiper_id,
            OrderStatus::RefundFailed,
        );

        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(refunding.clone())));
        repo.expect_update()
            .returning(move |_, _, _| Ok(refund_failed.clone()));

        let app = TestApp::new(default_state(repo));
        let req = json_request_internal_post(
            &format!("/internal/orders/{}/refund-confirmed", order_id),
            VALID_KEY,
            Some(json!({
                "success": false,
                "wallet_transaction_id": Uuid::new_v4(),
                "amount_refunded": 0_i64,
                "notes": "Gagal proses refund oleh sistem wallet",
            })),
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
    }

    #[tokio::test]
    async fn refund_confirmed_gagal_order_tidak_ditemukan_404() {
        setup_service_key();

        let order_id = Uuid::new_v4();
        let mut repo = MockOrderRepository::new();
        repo.expect_find_by_id().returning(|_| Ok(None));

        let app = TestApp::new(default_state(repo));
        let req = json_request_internal_post(
            &format!("/internal/orders/{}/refund-confirmed", order_id),
            VALID_KEY,
            Some(json!({
                "success": true,
                "wallet_transaction_id": Uuid::new_v4(),
                "amount_refunded": 11_000_i64,
            })),
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["success"], false);
    }
}
