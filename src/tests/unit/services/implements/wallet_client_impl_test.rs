#[cfg(test)]
mod tests {
    use crate::error::AppError;
    use crate::services::implements::wallet_client_impl::deduct_wallet;
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
}
