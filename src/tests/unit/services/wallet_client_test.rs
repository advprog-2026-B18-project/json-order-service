#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::services::wallet_client::{check_wallet, deduct_wallet, refund_wallet};

    // deduct_wallet berhasil (200)
    #[tokio::test]
    async fn test_deduct_wallet_berhasil() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/internal/wallets/deduct"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let (user_id, order_id) = (Uuid::new_v4(), Uuid::new_v4());
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("WALLET_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { deduct_wallet(user_id, order_id, 50_000, "Test deduct").await },
        )
        .await;

        assert!(result.is_ok());
    }

    // deduct_wallet saldo tidak cukup (422) → UnprocessableEntity
    #[tokio::test]
    async fn test_deduct_wallet_saldo_tidak_cukup() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/internal/wallets/deduct"))
            .respond_with(ResponseTemplate::new(422))
            .mount(&server)
            .await;

        let (user_id, order_id) = (Uuid::new_v4(), Uuid::new_v4());
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("WALLET_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { deduct_wallet(user_id, order_id, 50_000, "Test deduct").await },
        )
        .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::AppError::UnprocessableEntity(msg) => {
                assert!(msg.contains("Saldo tidak mencukupi"));
            }
            e => panic!("Expected UnprocessableEntity, got {:?}", e),
        }
    }

    // deduct_wallet user tidak ditemukan (404) → NotFound
    #[tokio::test]
    async fn test_deduct_wallet_user_tidak_ditemukan() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/internal/wallets/deduct"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let (user_id, order_id) = (Uuid::new_v4(), Uuid::new_v4());
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("WALLET_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { deduct_wallet(user_id, order_id, 50_000, "Test deduct").await },
        )
        .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::AppError::NotFound(_) => {}
            e => panic!("Expected NotFound, got {:?}", e),
        }
    }

    // deduct_wallet idempotent (409) → Ok
    #[tokio::test]
    async fn test_deduct_wallet_idempotent() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/internal/wallets/deduct"))
            .respond_with(ResponseTemplate::new(409))
            .mount(&server)
            .await;

        let (user_id, order_id) = (Uuid::new_v4(), Uuid::new_v4());
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("WALLET_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { deduct_wallet(user_id, order_id, 50_000, "Test deduct").await },
        )
        .await;

        assert!(result.is_ok()); // idempotent → ok
    }

    // refund_wallet berhasil (200)
    #[tokio::test]
    async fn test_refund_wallet_berhasil() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/internal/wallets/refund"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let (user_id, order_id) = (Uuid::new_v4(), Uuid::new_v4());
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("WALLET_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { refund_wallet(user_id, order_id, 50_000, "Test refund").await },
        )
        .await;

        assert!(result.is_ok());
    }

    // refund_wallet idempotent (409) → Ok
    #[tokio::test]
    async fn test_refund_wallet_idempotent() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/internal/wallets/refund"))
            .respond_with(ResponseTemplate::new(409))
            .mount(&server)
            .await;

        let (user_id, order_id) = (Uuid::new_v4(), Uuid::new_v4());
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("WALLET_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { refund_wallet(user_id, order_id, 50_000, "Test refund").await },
        )
        .await;

        assert!(result.is_ok());
    }

    // check_wallet saldo cukup (200 + is_sufficient=true) → Ok
    #[tokio::test]
    async fn test_check_wallet_saldo_cukup() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/internal/wallets/balance-check"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "is_sufficient": true })),
            )
            .mount(&server)
            .await;

        let user_id = Uuid::new_v4();
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("WALLET_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { check_wallet(user_id, 50_000).await },
        )
        .await;

        assert!(result.is_ok());
    }

    // check_wallet saldo tidak cukup (200 + is_sufficient=false) → UnprocessableEntity
    #[tokio::test]
    async fn test_check_wallet_saldo_tidak_cukup() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/internal/wallets/balance-check"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "is_sufficient": false })),
            )
            .mount(&server)
            .await;

        let user_id = Uuid::new_v4();
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("WALLET_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { check_wallet(user_id, 50_000).await },
        )
        .await;

        assert!(result.is_err());
    }
}
