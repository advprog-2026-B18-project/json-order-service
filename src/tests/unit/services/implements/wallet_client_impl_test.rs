#[cfg(test)]
mod tests {
    use crate::error::AppError;
    use crate::services::implements::wallet_client_impl::{
        check_wallet, deduct_wallet, earnings_wallet, refund_wallet, reverse_earnings,
    };
    use serde_json::json;
    use uuid::Uuid;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[serial_test::serial]
    #[tokio::test]
    async fn deduct_wallet_sukses_200() {
        let mock_server = MockServer::start().await;

        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
        }
        unsafe {
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }

        Mock::given(method("POST"))
            .and(path("/internal/wallets/deduct"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "transaction_id": "txn-abc-123"
            })))
            .mount(&mock_server)
            .await;

        let result =
            deduct_wallet(Uuid::new_v4(), Uuid::new_v4(), 50_000, "Pembayaran order").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().transaction_id, "txn-abc-123");
    }

    #[tokio::test]
    async fn deduct_wallet_gagal_422_saldo_tidak_cukup() {
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
        }

        Mock::given(method("POST"))
            .and(path("/internal/wallets/deduct"))
            .respond_with(ResponseTemplate::new(422).set_body_json(json!({})))
            .mount(&mock_server)
            .await;

        let result = deduct_wallet(Uuid::new_v4(), Uuid::new_v4(), 50_000, "Pembayaran").await;

        assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn deduct_wallet_idempotent_409() {
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
        }

        Mock::given(method("POST"))
            .and(path("/internal/wallets/deduct"))
            .respond_with(ResponseTemplate::new(409).set_body_json(json!({
                "transaction_id": "txn-existing"
            })))
            .mount(&mock_server)
            .await;

        let result = deduct_wallet(Uuid::new_v4(), Uuid::new_v4(), 50_000, "Pembayaran").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().transaction_id, "txn-existing");
    }

    // === Happy Path ===

    #[serial_test::serial]
    #[tokio::test]
    async fn test_refund_wallet_200_returns_transaction_id() {
        // Arrange
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("POST"))
            .and(path("/internal/wallets/refund"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "transaction_id": "refund-1"
            })))
            .mount(&mock_server)
            .await;

        // Act
        let result = refund_wallet(Uuid::new_v4(), Uuid::new_v4(), 50_000, "Refund").await;

        // Assert
        assert_eq!(result.unwrap().transaction_id, "refund-1");
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_check_wallet_sufficient_200_returns_ok() {
        // Arrange
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("GET"))
            .and(path("/internal/wallets/balance-check"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "is_sufficient": true
            })))
            .mount(&mock_server)
            .await;

        // Act
        let result = check_wallet(Uuid::new_v4(), 50_000).await;

        // Assert
        assert!(result.is_ok());
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_earnings_wallet_success_200_returns_transaction_id() {
        // Arrange
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("POST"))
            .and(path("/internal/wallets/earnings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "SUCCESS",
                "transaction_id": "earn-1"
            })))
            .mount(&mock_server)
            .await;

        // Act
        let result = earnings_wallet(Uuid::new_v4(), Uuid::new_v4(), "Earnings").await;

        // Assert
        assert_eq!(result.unwrap().transaction_id, "earn-1");
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_reverse_earnings_200_returns_ok() {
        // Arrange
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("POST"))
            .and(path("/internal/wallets/earnings/reverse"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&mock_server)
            .await;

        // Act
        let result = reverse_earnings(Uuid::new_v4(), Uuid::new_v4(), "txn-1", "Reverse").await;

        // Assert
        assert!(result.is_ok());
    }

    // === Error Path ===

    #[serial_test::serial]
    #[tokio::test]
    async fn test_deduct_wallet_404_returns_not_found() {
        // Arrange
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("POST"))
            .and(path("/internal/wallets/deduct"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({})))
            .mount(&mock_server)
            .await;

        // Act
        let result = deduct_wallet(Uuid::new_v4(), Uuid::new_v4(), 50_000, "Pembayaran").await;

        // Assert
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_refund_wallet_500_returns_internal() {
        // Arrange
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("POST"))
            .and(path("/internal/wallets/refund"))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({})))
            .mount(&mock_server)
            .await;

        // Act
        let result = refund_wallet(Uuid::new_v4(), Uuid::new_v4(), 50_000, "Refund").await;

        // Assert
        assert!(matches!(result, Err(AppError::Internal)));
    }

    // === Additional error branches ===

    #[serial_test::serial]
    #[tokio::test]
    async fn test_check_wallet_insufficient_200_returns_unprocessable_entity() {
        // Arrange
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("GET"))
            .and(path("/internal/wallets/balance-check"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "is_sufficient": false
            })))
            .mount(&mock_server)
            .await;

        // Act
        let result = check_wallet(Uuid::new_v4(), 50_000).await;

        // Assert
        assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_earnings_wallet_failed_status_returns_unprocessable_entity() {
        // Arrange
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("POST"))
            .and(path("/internal/wallets/earnings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "FAILED"
            })))
            .mount(&mock_server)
            .await;

        // Act
        let result = earnings_wallet(Uuid::new_v4(), Uuid::new_v4(), "Earnings").await;

        // Assert
        assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_reverse_earnings_422_returns_unprocessable_entity() {
        // Arrange
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("POST"))
            .and(path("/internal/wallets/earnings/reverse"))
            .respond_with(ResponseTemplate::new(422).set_body_json(json!({})))
            .mount(&mock_server)
            .await;

        // Act
        let result = reverse_earnings(Uuid::new_v4(), Uuid::new_v4(), "txn-1", "Reverse").await;

        // Assert
        assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_deduct_wallet_500_returns_internal() {
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("POST"))
            .and(path("/internal/wallets/deduct"))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({})))
            .mount(&mock_server)
            .await;
        let result = deduct_wallet(Uuid::new_v4(), Uuid::new_v4(), 50_000, "Pembayaran").await;
        assert!(matches!(result, Err(AppError::Internal)));
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_refund_wallet_409_returns_transaction_id() {
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("POST"))
            .and(path("/internal/wallets/refund"))
            .respond_with(ResponseTemplate::new(409).set_body_json(json!({
                "transaction_id": "txn-existing"
            })))
            .mount(&mock_server)
            .await;
        let result = refund_wallet(Uuid::new_v4(), Uuid::new_v4(), 50_000, "Refund").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().transaction_id, "txn-existing");
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_check_wallet_404_returns_ok() {
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("GET"))
            .and(path("/internal/wallets/balance-check"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({})))
            .mount(&mock_server)
            .await;
        let result = check_wallet(Uuid::new_v4(), 50_000).await;
        assert!(result.is_ok());
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_check_wallet_500_returns_internal() {
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("GET"))
            .and(path("/internal/wallets/balance-check"))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({})))
            .mount(&mock_server)
            .await;
        let result = check_wallet(Uuid::new_v4(), 50_000).await;
        assert!(matches!(result, Err(AppError::Internal)));
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_earnings_wallet_409_returns_conflict() {
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("POST"))
            .and(path("/internal/wallets/earnings"))
            .respond_with(ResponseTemplate::new(409).set_body_json(json!({})))
            .mount(&mock_server)
            .await;
        let result = earnings_wallet(Uuid::new_v4(), Uuid::new_v4(), "Earnings").await;
        assert!(matches!(result, Err(AppError::Conflict(_))));
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_earnings_wallet_404_returns_not_found() {
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("POST"))
            .and(path("/internal/wallets/earnings"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({
                "message": "Not found"
            })))
            .mount(&mock_server)
            .await;
        let result = earnings_wallet(Uuid::new_v4(), Uuid::new_v4(), "Earnings").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_earnings_wallet_500_returns_internal() {
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("POST"))
            .and(path("/internal/wallets/earnings"))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({})))
            .mount(&mock_server)
            .await;
        let result = earnings_wallet(Uuid::new_v4(), Uuid::new_v4(), "Earnings").await;
        assert!(matches!(result, Err(AppError::Internal)));
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_reverse_earnings_404_returns_not_found() {
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("POST"))
            .and(path("/internal/wallets/earnings/reverse"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({})))
            .mount(&mock_server)
            .await;
        let result = reverse_earnings(Uuid::new_v4(), Uuid::new_v4(), "txn-1", "Reverse").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_reverse_earnings_409_returns_ok() {
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("POST"))
            .and(path("/internal/wallets/earnings/reverse"))
            .respond_with(ResponseTemplate::new(409).set_body_json(json!({})))
            .mount(&mock_server)
            .await;
        let result = reverse_earnings(Uuid::new_v4(), Uuid::new_v4(), "txn-1", "Reverse").await;
        assert!(result.is_ok());
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_reverse_earnings_500_returns_internal() {
        let mock_server = MockServer::start().await;
        unsafe {
            std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
        }
        Mock::given(method("POST"))
            .and(path("/internal/wallets/earnings/reverse"))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({})))
            .mount(&mock_server)
            .await;
        let result = reverse_earnings(Uuid::new_v4(), Uuid::new_v4(), "txn-1", "Reverse").await;
        assert!(matches!(result, Err(AppError::Internal)));
    }
}
