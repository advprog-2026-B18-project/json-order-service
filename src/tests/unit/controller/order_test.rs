#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use chrono::Utc;
    use serde_json::json;
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::error::AppError;
    use crate::models::order::{Order, OrderStatus};
    use crate::ports::inventory_client::MockInventoryClient;
    use crate::ports::order_repository::MockOrderRepository;
    use crate::ports::order_status_history_repository::MockOrderStatusHistoryRepository;
    use crate::ports::wallet_client::MockWalletClient;
    use crate::state::AppState;

    use crate::tests::unit::controller::helper_test::{TestApp, json_request, make_test_token};

    pub fn setup_jwt_secret() {
        unsafe {
            std::env::set_var("JWT_SECRET", "dGVzdC1zZWNyZXQtdGVzdC1zZWNyZXQ=");
        }
    }

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
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn make_checkout_body(product_id: Uuid) -> serde_json::Value {
        json!({
            "product_id": product_id,
            "quantity": 1,
            "shipping_address": {
                "recipient_name": "Ahmad Fauzan",
                "phone_number": "081234567890",
                "street": "Jl. Mawar No. 12",
                "kelurahan": "Cipete Selatan",
                "kecamatan": "Cilandak",
                "city": "Jakarta Selatan",
                "province": "DKI Jakarta",
                "postal_code": "12410",
                "notes": null
            },
            "note_to_jastiper": null
        })
    }

    #[tokio::test]
    async fn checkout_sukses_201() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let product_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut inv = MockInventoryClient::new();
        let mut wallet = MockWalletClient::new();
        let mut repo = MockOrderRepository::new();

        inv.expect_fetch_product().returning(move |_| {
            Ok(json!({
                "jastiperId":  jastiper_id,
                "name":        "Snickers",
                "price":       10_000_i64,
                "service_fee": 1_000_i64,
            }))
        });
        inv.expect_reserve_stock().returning(|_, _, _| Ok(()));
        wallet.expect_check_wallet().returning(|_, _| Ok(()));

        let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
        repo.expect_create()
            .returning(move |_, _, _, _, _| Ok(order.clone()));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(inv),
            wallet_client: Arc::new(wallet),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(titipers_id, "TITIPERS");
        println!("DEBUG TEST - Token dibuat: {}", token);

        let req = json_request(
            "POST",
            "/orders",
            &token,
            Some(make_checkout_body(product_id)),
        );

        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["success"], true);
        assert!(body["data"]["order_id"].is_string());
    }

    #[tokio::test]
    async fn checkout_gagal_unauthorized_tanpa_token_401() {
        setup_jwt_secret();

        let app = TestApp::new(AppState {
            order_repo: Arc::new(MockOrderRepository::new()),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/orders")
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                make_checkout_body(Uuid::new_v4()).to_string(),
            ))
            .unwrap();

        let (status, _) = app.send(req).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn checkout_gagal_produk_tidak_ditemukan_404() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let mut inv = MockInventoryClient::new();

        inv.expect_fetch_product()
            .returning(|_| Err(AppError::NotFound("Produk tidak ditemukan".to_string())));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(MockOrderRepository::new()),
            inventory_client: Arc::new(inv),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(titipers_id, "TITIPERS");
        let req = json_request(
            "POST",
            "/orders",
            &token,
            Some(make_checkout_body(Uuid::new_v4())),
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["success"], false);
    }

    #[tokio::test]
    async fn checkout_gagal_saldo_tidak_cukup_422() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();

        let mut inv = MockInventoryClient::new();
        let mut wallet = MockWalletClient::new();

        inv.expect_fetch_product().returning(move |_| {
            Ok(json!({ "jastiperId": jastiper_id, "price": 10_000_i64, "service_fee": 1_000_i64 }))
        });
        inv.expect_reserve_stock().returning(|_, _, _| Ok(()));
        inv.expect_release_stock().returning(|_, _, _| Ok(()));

        wallet.expect_check_wallet().returning(|_, _| {
            Err(AppError::UnprocessableEntity(
                "Saldo tidak cukup".to_string(),
            ))
        });

        let app = TestApp::new(AppState {
            order_repo: Arc::new(MockOrderRepository::new()),
            inventory_client: Arc::new(inv),
            wallet_client: Arc::new(wallet),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(titipers_id, "TITIPERS");
        let req = json_request(
            "POST",
            "/orders",
            &token,
            Some(make_checkout_body(Uuid::new_v4())),
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["success"], false);
    }

    #[tokio::test]
    async fn checkout_gagal_body_tidak_valid_422() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();

        let app = TestApp::new(AppState {
            order_repo: Arc::new(MockOrderRepository::new()),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(titipers_id, "TITIPERS");
        let req = json_request("POST", "/orders", &token, Some(json!({"invalid": "body"})));
        let (status, _) = app.send(req).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn get_order_sukses_200() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);

        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(order.clone())));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(titipers_id, "TITIPERS");
        let req = json_request("GET", &format!("/orders/{}", order_id), &token, None);
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["order_id"], order_id.to_string());
    }

    #[tokio::test]
    async fn get_order_tidak_ditemukan_404() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        repo.expect_find_by_id().returning(|_| Ok(None));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(titipers_id, "TITIPERS");
        let req = json_request("GET", &format!("/orders/{}", Uuid::new_v4()), &token, None);
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["success"], false);
    }

    #[tokio::test]
    async fn get_order_bukan_pemilik_403() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let orang_lain = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(order.clone())));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(orang_lain, "TITIPERS");
        let req = json_request("GET", &format!("/orders/{}", order_id), &token, None);
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["success"], false);
    }

    #[tokio::test]
    async fn payment_sukses_200() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let mut wallet = MockWalletClient::new();

        let pending = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
        let paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);

        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(pending.clone())));
        repo.expect_update()
            .returning(move |_, _, _| Ok(paid.clone()));
        wallet.expect_deduct_wallet().returning(|_, _, _, _| Ok(()));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(wallet),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(titipers_id, "Titipers");
        let req = json_request(
            "PATCH",
            &format!("/orders/{}/payment", order_id),
            &token,
            None,
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
    }

    #[tokio::test]
    async fn payment_gagal_order_sudah_paid_409() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);

        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(paid.clone())));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(titipers_id, "Titipers");
        let req = json_request(
            "PATCH",
            &format!("/orders/{}/payment", order_id),
            &token,
            None,
        );
        let (status, _) = app.send(req).await;

        assert!(
            status == StatusCode::CONFLICT || status == StatusCode::UNPROCESSABLE_ENTITY,
            "expected 409 or 422, got {} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        );
    }

    #[tokio::test]
    async fn confirm_order_sukses_200() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let shipped = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);
        let completed = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);

        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(shipped.clone())));
        repo.expect_update()
            .returning(move |_, _, _| Ok(completed.clone()));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(titipers_id, "Titipers");
        let req = json_request(
            "PATCH",
            &format!("/orders/{}/confirm", order_id),
            &token,
            None,
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
    }

    #[tokio::test]
    async fn confirm_order_bukan_titipers_pemilik_403() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let orang_lain = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let shipped = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);
        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(shipped.clone())));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(orang_lain, "Titipers");
        let req = json_request(
            "PATCH",
            &format!("/orders/{}/confirm", order_id),
            &token,
            None,
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["success"], false);
    }

    #[tokio::test]
    async fn purchased_sukses_200() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
        let purchased = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);

        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(paid.clone())));
        repo.expect_update()
            .returning(move |_, _, _| Ok(purchased.clone()));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(jastiper_id, "Jastiper");
        let req = json_request(
            "PATCH",
            &format!("/orders/{}/purchased", order_id),
            &token,
            None,
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
    }

    #[tokio::test]
    async fn purchased_bukan_jastiper_pemilik_403() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let orang_lain = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(paid.clone())));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(orang_lain, "Jastiper");
        let req = json_request(
            "PATCH",
            &format!("/orders/{}/purchased", order_id),
            &token,
            None,
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["success"], false);
    }

    #[tokio::test]
    async fn shipped_sukses_200() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let purchased = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);
        let mut shipped = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);
        shipped.tracking_number = Some("JNE-999".to_string());
        shipped.courier = Some("JNE".to_string());

        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(purchased.clone())));
        repo.expect_update()
            .returning(move |_, _, _| Ok(shipped.clone()));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(jastiper_id, "Jastiper");
        let req = json_request(
            "PATCH",
            &format!("/orders/{}/shipped", order_id),
            &token,
            Some(json!({ "tracking_number": "JNE-999", "courier": "JNE" })),
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["tracking_number"], "JNE-999");
        assert_eq!(body["data"]["courier"], "JNE");
    }

    #[tokio::test]
    async fn shipped_gagal_tanpa_tracking_number_422() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);
        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(order.clone())));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(jastiper_id, "Jastiper");
        let req = json_request(
            "PATCH",
            &format!("/orders/{}/shipped", order_id),
            &token,
            Some(json!({ "tracking_number": null, "courier": "JNE" })),
        );
        let (status, _) = app.send(req).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn get_order_history_sukses_200() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let mut history_repo = MockOrderStatusHistoryRepository::new();

        let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);
        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(order.clone())));
        history_repo
            .expect_get_status_history()
            .returning(|_| Ok(vec![]));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(history_repo),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(titipers_id, "TITIPERS");
        let req = json_request(
            "GET",
            &format!("/orders/{}/history", order_id),
            &token,
            None,
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        assert_eq!(body["message"], "Riwayat ditemukan");
    }

    #[tokio::test]
    async fn get_order_history_bukan_pemilik_403() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let orang_lain = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(order.clone())));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(orang_lain, "Titipers");
        let req = json_request(
            "GET",
            &format!("/orders/{}/history", order_id),
            &token,
            None,
        );
        let (status, _) = app.send(req).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn cancel_order_sukses_titipers_200() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut repo = MockOrderRepository::new();
        let mut inv = MockInventoryClient::new();
        let mut wallet = MockWalletClient::new();

        let pending = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
        let mut cancelled = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);
        cancelled.product_snapshot = json!({ "product_id": Uuid::new_v4() });

        repo.expect_find_by_id()
            .returning(move |_| Ok(Some(pending.clone())));
        repo.expect_update()
            .returning(move |_, _, _| Ok(cancelled.clone()));
        inv.expect_release_stock().returning(|_, _, _| Ok(()));
        wallet.expect_refund_wallet().returning(|_, _, _, _| Ok(()));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(inv),
            wallet_client: Arc::new(wallet),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(jastiper_id, "JASTIPER");
        let req = json_request(
            "POST",
            &format!("/orders/{}/cancel", order_id),
            &token,
            Some(json!({ "cancellation_reason": "Tidak jadi beli" })),
        );
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        assert_eq!(body["message"], "Pesanan berhasil dibatalkan");
    }

    #[tokio::test]
    async fn cancel_order_sukses_oleh_titipers_200() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut order_repo = MockOrderRepository::new();
        let mut inventory_client = MockInventoryClient::new();
        let mut wallet_client = MockWalletClient::new();

        order_repo
            .expect_find_by_id()
            .withf(move |id| *id == order_id)
            .returning(move |_| {
                Ok(Some(Order {
                    order_id,
                    titipers_id,
                    jastiper_id,
                    product_id: Uuid::new_v4(),
                    quantity: 2,
                    unit_price: 0,
                    service_fee: 0,
                    total_price: 150000,
                    status: OrderStatus::Paid,
                    shipping_address: Default::default(),
                    note_to_jastiper: None,
                    tracking_number: None,
                    courier: None,
                    cancellation_reason: None,
                    cancelled_by: None,
                    product_snapshot: json!({}),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    completed_at: None,
                }))
            });

        order_repo
            .expect_update()
            .withf(move |id, new_status, _params| {
                *id == order_id && *new_status == OrderStatus::Refunding
            })
            .returning(move |_, _, _| {
                Ok(Order {
                    order_id,
                    titipers_id,
                    jastiper_id: Uuid::new_v4(),
                    product_id: Uuid::new_v4(),
                    quantity: 2,
                    unit_price: 0,
                    service_fee: 0,
                    total_price: 150000,
                    status: OrderStatus::Refunding,
                    shipping_address: Default::default(),
                    note_to_jastiper: None,
                    tracking_number: None,
                    courier: None,
                    cancellation_reason: None,
                    cancelled_by: None,
                    product_snapshot: json!({}),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    completed_at: None,
                })
            });

        inventory_client
            .expect_release_stock()
            .returning(|_, _, _| Ok(()));

        wallet_client
            .expect_refund_wallet()
            .returning(|_, _, _, _| Ok(()));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(order_repo),
            inventory_client: Arc::new(inventory_client),
            wallet_client: Arc::new(wallet_client),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(jastiper_id, "JASTIPER");

        let req = json_request(
            "POST",
            &format!("/orders/{}/cancel", order_id),
            &token,
            Some(json!({
                "cancellation_reason": "Barang tidak sesuai deskripsi"
            })),
        );

        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.get("data").is_some());
        assert_eq!(body["data"]["status"], "REFUNDING");
    }

    #[tokio::test]
    async fn cancel_order_gagal_role_jastiper_403() {
        setup_jwt_secret();

        let jastiper_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let mut order_repo = MockOrderRepository::new();

        order_repo
            .expect_find_by_id()
            .withf(move |id| *id == order_id)
            .returning(move |_| {
                Ok(Some(Order {
                    order_id,
                    titipers_id: Uuid::new_v4(),
                    jastiper_id,
                    product_id: Uuid::new_v4(),
                    quantity: 1,
                    unit_price: 0,
                    service_fee: 0,
                    total_price: 100_000,
                    status: OrderStatus::Paid,
                    shipping_address: Default::default(),
                    note_to_jastiper: None,
                    tracking_number: None,
                    courier: None,
                    cancellation_reason: None,
                    cancelled_by: None,
                    product_snapshot: json!({}),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    completed_at: None,
                }))
            });

        order_repo
            .expect_update()
            .withf(move |id, new_status, _params| {
                *id == order_id
                    && matches!(new_status, OrderStatus::Refunding | OrderStatus::Cancelled)
            })
            .returning(move |_, _, _| {
                Err(AppError::Forbidden(
                    "Hanya titipers yang boleh cancel order".to_string(),
                ))
            });

        let app = TestApp::new(AppState {
            order_repo: Arc::new(order_repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(jastiper_id, "JASTIPER");

        let req = json_request(
            "POST",
            &format!("/orders/{}/cancel", order_id),
            &token,
            Some(json!({ "cancellation_reason": "Salah kirim" })),
        );

        let (status, _) = app.send(req).await;

        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn my_purchases_sukses_200() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let mut repo = MockOrderRepository::new();

        repo.expect_find_all().returning(|_, _| Ok((vec![], 0)));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(titipers_id, "Titipers");
        let req = json_request("GET", "/orders/my/purchases", &token, None);
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        assert!(body["pagination"]["total_items"].is_number());
        assert!(body["pagination"]["page"].is_number());
        assert!(body["pagination"]["limit"].is_number());
        assert!(body["pagination"]["total_pages"].is_number());
        assert_eq!(body["pagination"]["total_items"], 0);
    }

    #[tokio::test]
    async fn my_purchases_dengan_query_params_200() {
        setup_jwt_secret();

        let titipers_id = Uuid::new_v4();
        let mut repo = MockOrderRepository::new();

        repo.expect_find_all().returning(|_, _| Ok((vec![], 0)));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(titipers_id, "Titipers");
        let req = json_request("GET", "/orders/my/purchases?page=2&limit=5", &token, None);
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["pagination"]["page"], 2);
        assert_eq!(body["pagination"]["limit"], 5);
    }

    #[tokio::test]
    async fn my_sales_sukses_200() {
        setup_jwt_secret();

        let jastiper_id = Uuid::new_v4();
        let mut repo = MockOrderRepository::new();

        repo.expect_find_all().returning(|_, _| Ok((vec![], 0)));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(jastiper_id, "Jastiper");
        let req = json_request("GET", "/orders/my/sales", &token, None);
        let (status, body) = app.send(req).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        assert_eq!(body["message"], "Daftar pesanan masuk ditemukan");
        assert!(body["pagination"].is_object());
    }

    #[tokio::test]
    async fn my_sales_gagal_db_error_500() {
        setup_jwt_secret();

        let jastiper_id = Uuid::new_v4();
        let mut repo = MockOrderRepository::new();

        repo.expect_find_all()
            .returning(|_, _| Err(AppError::Internal));

        let app = TestApp::new(AppState {
            order_repo: Arc::new(repo),
            inventory_client: Arc::new(MockInventoryClient::new()),
            wallet_client: Arc::new(MockWalletClient::new()),
            order_status_history_repo: Arc::new(MockOrderStatusHistoryRepository::new()),
            rating_product_repo: Arc::new(
                crate::ports::rating_product_repository::MockRatingProductRepository::new(),
            ),
            rating_jastiper_repo: Arc::new(
                crate::ports::rating_jastiper_repository::MockRatingJastiperRepository::new(),
            ),
            auth_client: Arc::new(crate::ports::auth_client::MockAuthClient::new()),
        });

        let token = make_test_token(jastiper_id, "Jastiper");
        let req = json_request("GET", "/orders/my/sales", &token, None);
        let (status, _) = app.send(req).await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }
}
