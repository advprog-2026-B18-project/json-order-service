use crate::error::AppError;
use crate::services::implements::inventory_client_impl::{
    fetch_product, release_stock, reserve_stock,
};
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[serial_test::serial]
#[tokio::test]
async fn reserve_stock_sukses_200() {
    let mock_server = MockServer::start().await;
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", mock_server.uri());
    }
    unsafe {
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }

    Mock::given(method("POST"))
        .and(path_regex(r"/internal/products/.+/stock/reserve"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&mock_server)
        .await;

    let result = reserve_stock(Uuid::new_v4(), Uuid::new_v4(), 2).await;
    assert!(result.is_ok());
}

#[serial_test::serial]
#[tokio::test]
async fn release_stock_404_non_fatal_untuk_kompensasi() {
    let mock_server = MockServer::start().await;
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", mock_server.uri());
    }
    unsafe {
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }

    let product_id = Uuid::new_v4();

    Mock::given(method("POST"))
        .and(path(format!(
            "/internal/products/{}/stock/release",
            product_id
        )))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({})))
        .expect(1)
        .mount(&mock_server)
        .await;

    let result = release_stock(product_id, Uuid::new_v4(), 1).await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[serial_test::serial]
#[tokio::test]
async fn fetch_product_sukses_200() {
    let mock_server = MockServer::start().await;
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", mock_server.uri());
    }

    let product_data = json!({ "name": "Snickers", "price": 10_000 });

    Mock::given(method("GET"))
        .and(path_regex(r"/products/.+"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": product_data
        })))
        .mount(&mock_server)
        .await;

    let result = fetch_product(Uuid::new_v4()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap()["name"], "Snickers");
}
