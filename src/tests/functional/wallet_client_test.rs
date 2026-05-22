use crate::services::adapters::wallet_client_adapt::HttpWalletClient;
use crate::services::wallet_client::WalletClient;
use serde_json::json;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn set_env(mock_server: &MockServer) {
    unsafe {
        std::env::set_var("WALLET_SERVICE_URL", mock_server.uri());
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }
}

// === Happy Path ===
#[serial_test::serial]
#[tokio::test]
async fn test_http_wallet_client_deduct_wallet_success_delegates_to_impl() {
    let mock_server = MockServer::start().await;
    set_env(&mock_server);

    Mock::given(method("POST"))
        .and(path("/internal/wallets/deduct"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "transaction_id": "deduct-1"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = HttpWalletClient;

    let result = client
        .deduct_wallet(Uuid::new_v4(), Uuid::new_v4(), 10_000, "payment")
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().transaction_id, "deduct-1");
}

#[serial_test::serial]
#[tokio::test]
async fn test_http_wallet_client_refund_wallet_success_delegates_to_impl() {
    let mock_server = MockServer::start().await;
    set_env(&mock_server);

    Mock::given(method("POST"))
        .and(path("/internal/wallets/refund"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "transaction_id": "refund-1"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = HttpWalletClient;

    let result = client
        .refund_wallet(Uuid::new_v4(), Uuid::new_v4(), 10_000, "refund")
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().transaction_id, "refund-1");
}

#[serial_test::serial]
#[tokio::test]
async fn test_http_wallet_client_check_wallet_success_delegates_to_impl() {
    let mock_server = MockServer::start().await;
    set_env(&mock_server);

    Mock::given(method("GET"))
        .and(path("/internal/wallets/balance-check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "is_sufficient": true
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = HttpWalletClient;

    let result = client.check_wallet(Uuid::new_v4(), 10_000).await;

    assert!(result.is_ok());
}

#[serial_test::serial]
#[tokio::test]
async fn test_http_wallet_client_earnings_wallet_success_delegates_to_impl() {
    let mock_server = MockServer::start().await;
    set_env(&mock_server);

    Mock::given(method("POST"))
        .and(path("/internal/wallets/earnings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "SUCCESS",
            "transaction_id": "earn-1"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = HttpWalletClient;

    let result = client
        .earnings_wallet(Uuid::new_v4(), Uuid::new_v4(), "earnings")
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().transaction_id, "earn-1");
}

#[serial_test::serial]
#[tokio::test]
async fn test_http_wallet_client_reverse_earnings_success_delegates_to_impl() {
    let mock_server = MockServer::start().await;
    set_env(&mock_server);

    Mock::given(method("POST"))
        .and(path("/internal/wallets/earnings/reverse"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = HttpWalletClient;

    let result = client
        .reverse_earnings(Uuid::new_v4(), Uuid::new_v4(), "earn-1", "reverse")
        .await;

    assert!(result.is_ok());
}
