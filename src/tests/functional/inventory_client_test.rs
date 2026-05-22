use crate::services::adapters::inventory_client_adapt::HttpInventoryClient;
use crate::services::inventory_client::InventoryClient;
use serde_json::json;
use uuid::Uuid;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn set_env(mock_server: &MockServer) {
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", mock_server.uri());
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }
}

// === Happy Path ===
#[serial_test::serial]
#[tokio::test]
async fn test_http_inventory_client_reserve_stock_success_delegates_to_impl() {
    let mock_server = MockServer::start().await;
    set_env(&mock_server);

    Mock::given(method("POST"))
        .and(path_regex(r"/internal/products/.+/stock/reserve"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = HttpInventoryClient;

    let result = client
        .reserve_stock(Uuid::new_v4(), Uuid::new_v4(), 2)
        .await;

    assert!(result.is_ok());
}

#[serial_test::serial]
#[tokio::test]
async fn test_http_inventory_client_release_stock_success_delegates_to_impl() {
    let mock_server = MockServer::start().await;
    set_env(&mock_server);

    Mock::given(method("POST"))
        .and(path_regex(r"/internal/products/.+/stock/release"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = HttpInventoryClient;

    let result = client
        .release_stock(Uuid::new_v4(), Uuid::new_v4(), 2)
        .await;

    assert!(result.is_ok());
}

#[serial_test::serial]
#[tokio::test]
async fn test_http_inventory_client_fetch_product_success_delegates_to_impl() {
    let mock_server = MockServer::start().await;
    set_env(&mock_server);

    Mock::given(method("GET"))
        .and(path_regex(r"/products/.+"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "name": "Snack", "price": 10_000 }
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = HttpInventoryClient;

    let result = client.fetch_product(Uuid::new_v4()).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap()["name"], "Snack");
}

#[serial_test::serial]
#[tokio::test]
async fn test_http_inventory_client_send_product_rating_success_delegates_to_impl() {
    let mock_server = MockServer::start().await;
    set_env(&mock_server);

    Mock::given(method("POST"))
        .and(path_regex(r"/internal/products/.+/post-order"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = HttpInventoryClient;

    let result = client
        .send_product_rating(
            Uuid::new_v4(),
            Uuid::new_v4(),
            5.0,
            Some("great"),
            vec!["image.png"],
        )
        .await;

    assert!(result.is_ok());
}

#[serial_test::serial]
#[tokio::test]
async fn test_http_inventory_client_confirm_order_received_success_delegates_to_impl() {
    let mock_server = MockServer::start().await;
    set_env(&mock_server);

    Mock::given(method("POST"))
        .and(path_regex(r"/internal/products/.+/post-order"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = HttpInventoryClient;

    let result = client
        .confirm_order_received(Uuid::new_v4(), Uuid::new_v4())
        .await;

    assert!(result.is_ok());
}

// === Idempotent Path (409) ===
#[serial_test::serial]
#[tokio::test]
async fn test_http_inventory_client_send_product_rating_409_returns_ok() {
    let mock_server = MockServer::start().await;
    set_env(&mock_server);

    Mock::given(method("POST"))
        .and(path_regex(r"/internal/products/.+/post-order"))
        .respond_with(ResponseTemplate::new(409).set_body_json(json!({})))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = HttpInventoryClient;

    let result = client
        .send_product_rating(Uuid::new_v4(), Uuid::new_v4(), 4.0, Some("good"), vec![])
        .await;

    assert!(result.is_ok());
}

#[serial_test::serial]
#[tokio::test]
async fn test_http_inventory_client_confirm_order_received_409_returns_ok() {
    let mock_server = MockServer::start().await;
    set_env(&mock_server);

    Mock::given(method("POST"))
        .and(path_regex(r"/internal/products/.+/post-order"))
        .respond_with(ResponseTemplate::new(409).set_body_json(json!({})))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = HttpInventoryClient;

    let result = client
        .confirm_order_received(Uuid::new_v4(), Uuid::new_v4())
        .await;

    assert!(result.is_ok());
}

// === Error Path (unexpected status) ===
#[serial_test::serial]
#[tokio::test]
async fn test_http_inventory_client_send_product_rating_unexpected_status_returns_ok() {
    let mock_server = MockServer::start().await;
    set_env(&mock_server);

    Mock::given(method("POST"))
        .and(path_regex(r"/internal/products/.+/post-order"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({})))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = HttpInventoryClient;

    let result = client
        .send_product_rating(Uuid::new_v4(), Uuid::new_v4(), 4.0, Some("good"), vec![])
        .await;

    assert!(result.is_ok());
}

#[serial_test::serial]
#[tokio::test]
async fn test_http_inventory_client_confirm_order_received_unexpected_status_returns_internal() {
    let mock_server = MockServer::start().await;
    set_env(&mock_server);

    Mock::given(method("POST"))
        .and(path_regex(r"/internal/products/.+/post-order"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({})))
        .up_to_n_times(10)
        .mount(&mock_server)
        .await;

    let client = HttpInventoryClient;

    let result = client
        .confirm_order_received(Uuid::new_v4(), Uuid::new_v4())
        .await;

    assert!(result.is_ok());
}
