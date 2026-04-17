use mockito::Server;
use uuid::Uuid;

#[tokio::test]
#[serial_test::serial]
async fn deduct_wallet_sukses() {
    let mut server = Server::new_async().await;

    server
        .mock("POST", "/internal/wallets/deduct")
        .with_status(200)
        .create_async()
        .await;

    temp_env::async_with_vars(
        [
            ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ("WALLET_SERVICE_URL", Some(server.url().as_str())),
        ],
        async {
            let result = crate::services::wallet_client::deduct_wallet(
                Uuid::new_v4(),
                Uuid::new_v4(),
                50_000,
                "Pembayaran order",
            )
            .await;
            assert!(result.is_ok());
        },
    )
    .await;
}

#[tokio::test]
#[serial_test::serial]
async fn deduct_wallet_idempotent_409() {
    let mut server = Server::new_async().await;

    server
        .mock("POST", "/internal/wallets/deduct")
        .with_status(409)
        .create_async()
        .await;

    temp_env::async_with_vars(
        [
            ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ("WALLET_SERVICE_URL", Some(server.url().as_str())),
        ],
        async {
            let result = crate::services::wallet_client::deduct_wallet(
                Uuid::new_v4(),
                Uuid::new_v4(),
                50_000,
                "Pembayaran order",
            )
            .await;
            assert!(result.is_ok());
        },
    )
    .await
}

#[tokio::test]
#[serial_test::serial]
async fn deduct_wallet_gagal_user_tidak_ditemukan_404() {
    let mut server = Server::new_async().await;

    server
        .mock("POST", "/internal/wallets/deduct")
        .with_status(404)
        .create_async()
        .await;

    temp_env::async_with_vars(
        [
            ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ("WALLET_SERVICE_URL", Some(server.url().as_str())),
        ],
        async {
            let result = crate::services::wallet_client::deduct_wallet(
                Uuid::new_v4(),
                Uuid::new_v4(),
                50_000,
                "Pembayaran",
            )
            .await;

            assert!(matches!(result, Err(crate::error::AppError::NotFound(_))));
        },
    )
    .await
}

#[tokio::test]
#[serial_test::serial]
async fn deduct_wallet_gagal_saldo_tidak_cukup_422() {
    let mut server = Server::new_async().await;

    server
        .mock("POST", "/internal/wallets/deduct")
        .with_status(422)
        .create_async()
        .await;

    temp_env::async_with_vars(
        [
            ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ("WALLET_SERVICE_URL", Some(server.url().as_str())),
        ],
        async {
            let result = crate::services::wallet_client::deduct_wallet(
                Uuid::new_v4(),
                Uuid::new_v4(),
                9_999_999,
                "Pembayaran",
            )
            .await;

            assert!(matches!(
                result,
                Err(crate::error::AppError::UnprocessableEntity(_))
            ));
        },
    )
    .await
}

#[tokio::test]
#[serial_test::serial]
async fn deduct_wallet_gagal_unexpected_status_500() {
    let mut server = Server::new_async().await;

    server
        .mock("POST", "/internal/wallets/deduct")
        .with_status(500)
        .create_async()
        .await;

    temp_env::async_with_vars(
        [
            ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ("WALLET_SERVICE_URL", Some(server.url().as_str())),
        ],
        async {
            let result = crate::services::wallet_client::deduct_wallet(
                Uuid::new_v4(),
                Uuid::new_v4(),
                50_000,
                "Pembayaran",
            )
            .await;

            assert!(matches!(result, Err(crate::error::AppError::Internal)));
        },
    )
    .await
}

#[tokio::test]
#[serial_test::serial]
async fn refund_wallet_sukses() {
    let mut server = Server::new_async().await;

    server
        .mock("POST", "/internal/wallets/refund")
        .with_status(200)
        .create_async()
        .await;

    temp_env::async_with_vars(
        [
            ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ("WALLET_SERVICE_URL", Some(server.url().as_str())),
        ],
        async {
            let result = crate::services::wallet_client::refund_wallet(
                Uuid::new_v4(),
                Uuid::new_v4(),
                50_000,
                "Refund order dibatalkan",
            )
            .await;

            assert!(result.is_ok());
        },
    )
    .await
}

#[tokio::test]
#[serial_test::serial]
async fn refund_wallet_idempotent_409() {
    let mut server = Server::new_async().await;

    server
        .mock("POST", "/internal/wallets/refund")
        .with_status(409)
        .create_async()
        .await;

    temp_env::async_with_vars(
        [
            ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ("WALLET_SERVICE_URL", Some(server.url().as_str())),
        ],
        async {
            let result = crate::services::wallet_client::refund_wallet(
                Uuid::new_v4(),
                Uuid::new_v4(),
                50_000,
                "Refund",
            )
            .await;

            assert!(result.is_ok());
        },
    )
    .await
}

#[tokio::test]
#[serial_test::serial]
async fn refund_wallet_gagal_unexpected_status_500() {
    let mut server = Server::new_async().await;

    server
        .mock("POST", "/internal/wallets/refund")
        .with_status(500)
        .create_async()
        .await;

    temp_env::async_with_vars(
        [
            ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ("WALLET_SERVICE_URL", Some(server.url().as_str())),
        ],
        async {
            let result = crate::services::wallet_client::refund_wallet(
                Uuid::new_v4(),
                Uuid::new_v4(),
                50_000,
                "Refund",
            )
            .await;

            assert!(matches!(result, Err(crate::error::AppError::Internal)));
        },
    )
    .await
}

#[tokio::test]
#[serial_test::serial]
async fn check_wallet_sukses_saldo_cukup() {
    let mut server = Server::new_async().await;

    server
        .mock("GET", "/internal/wallets/balance-check")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"is_sufficient": true}"#)
        .create_async()
        .await;

    temp_env::async_with_vars(
        [
            ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ("WALLET_SERVICE_URL", Some(server.url().as_str())),
        ],
        async {
            let result = crate::services::wallet_client::check_wallet(Uuid::new_v4(), 50_000).await;

            assert!(result.is_ok());
        },
    )
    .await
}

#[tokio::test]
#[serial_test::serial]
async fn check_wallet_gagal_saldo_tidak_cukup() {
    let mut server = Server::new_async().await;

    server
        .mock("GET", "/internal/wallets/balance-check")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"is_sufficient": false}"#)
        .create_async()
        .await;

    temp_env::async_with_vars(
        [
            ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ("WALLET_SERVICE_URL", Some(server.url().as_str())),
        ],
        async {
            let result =
                crate::services::wallet_client::check_wallet(Uuid::new_v4(), 9_999_999).await;

            assert!(matches!(
                result,
                Err(crate::error::AppError::UnprocessableEntity(_))
            ));
        },
    )
    .await
}

#[tokio::test]
#[serial_test::serial]
async fn check_wallet_user_tidak_ditemukan_404_tetap_ok() {
    let mut server = Server::new_async().await;

    server
        .mock("GET", "/internal/wallets/balance-check")
        .with_status(404)
        .create_async()
        .await;

    temp_env::async_with_vars(
        [
            ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ("WALLET_SERVICE_URL", Some(server.url().as_str())),
        ],
        async {
            let result = crate::services::wallet_client::check_wallet(Uuid::new_v4(), 50_000).await;

            assert!(result.is_err());
        },
    )
    .await
}

#[tokio::test]
#[serial_test::serial]
async fn check_wallet_gagal_unexpected_status_500() {
    let mut server = Server::new_async().await;

    server
        .mock("GET", "/internal/wallets/balance-check")
        .with_status(500)
        .create_async()
        .await;

    temp_env::async_with_vars(
        [
            ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ("WALLET_SERVICE_URL", Some(server.url().as_str())),
        ],
        async {
            let result = crate::services::wallet_client::check_wallet(Uuid::new_v4(), 50_000).await;

            assert!(matches!(result, Err(crate::error::AppError::Internal)));
        },
    )
    .await
}
