use crate::error::AppError;
use crate::services::implements::inventory_client_impl::{
    confirm_order_received, fetch_product, release_stock, reserve_stock, send_product_rating,
};
use crate::services::inventory_client::InventoryClient;
use serde_json::json;
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

// === Error Path ===

#[serial_test::serial]
#[tokio::test]
async fn test_reserve_stock_409_returns_conflict() {
    // Arrange
    let mock_server = MockServer::start().await;
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", mock_server.uri());
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }
    Mock::given(method("POST"))
        .and(path_regex(r"/internal/products/.+/stock/reserve"))
        .respond_with(ResponseTemplate::new(409).set_body_json(json!({})))
        .mount(&mock_server)
        .await;

    // Act
    let result = reserve_stock(Uuid::new_v4(), Uuid::new_v4(), 2).await;

    // Assert
    assert!(matches!(result, Err(AppError::Conflict(_))));
}

#[serial_test::serial]
#[tokio::test]
async fn test_reserve_stock_422_returns_unprocessable_entity() {
    // Arrange
    let mock_server = MockServer::start().await;
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", mock_server.uri());
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }
    Mock::given(method("POST"))
        .and(path_regex(r"/internal/products/.+/stock/reserve"))
        .respond_with(ResponseTemplate::new(422).set_body_json(json!({})))
        .mount(&mock_server)
        .await;

    // Act
    let result = reserve_stock(Uuid::new_v4(), Uuid::new_v4(), 2).await;

    // Assert
    assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
}

#[serial_test::serial]
#[tokio::test]
async fn test_reserve_stock_500_returns_internal() {
    // Arrange
    let mock_server = MockServer::start().await;
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", mock_server.uri());
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }
    Mock::given(method("POST"))
        .and(path_regex(r"/internal/products/.+/stock/reserve"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({})))
        .mount(&mock_server)
        .await;

    // Act
    let result = reserve_stock(Uuid::new_v4(), Uuid::new_v4(), 2).await;

    // Assert
    assert!(matches!(result, Err(AppError::Internal)));
}

#[serial_test::serial]
#[tokio::test]
async fn test_fetch_product_404_returns_not_found() {
    // Arrange
    let mock_server = MockServer::start().await;
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", mock_server.uri());
    }
    Mock::given(method("GET"))
        .and(path_regex(r"/products/.+"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({})))
        .mount(&mock_server)
        .await;

    // Act
    let result = fetch_product(Uuid::new_v4()).await;

    // Assert
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[serial_test::serial]
#[tokio::test]
async fn test_fetch_product_422_returns_unprocessable_entity() {
    // Arrange
    let mock_server = MockServer::start().await;
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", mock_server.uri());
    }
    Mock::given(method("GET"))
        .and(path_regex(r"/products/.+"))
        .respond_with(ResponseTemplate::new(422).set_body_json(json!({})))
        .mount(&mock_server)
        .await;

    // Act
    let result = fetch_product(Uuid::new_v4()).await;

    // Assert
    assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
}

#[serial_test::serial]
#[tokio::test]
async fn test_fetch_product_malformed_json_returns_internal() {
    // Arrange
    let mock_server = MockServer::start().await;
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", mock_server.uri());
    }
    Mock::given(method("GET"))
        .and(path_regex(r"/products/.+"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not-json"))
        .mount(&mock_server)
        .await;

    // Act
    let result = fetch_product(Uuid::new_v4()).await;

    // Assert
    assert!(matches!(result, Err(AppError::Internal)));
}

#[serial_test::serial]
#[tokio::test]
async fn test_send_product_rating_404_is_ok() {
    // Arrange
    let mock_server = MockServer::start().await;
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", mock_server.uri());
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }
    Mock::given(method("POST"))
        .and(path_regex(r"/internal/products/.+/post-order"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({})))
        .mount(&mock_server)
        .await;

    // Act
    let result = send_product_rating(Uuid::new_v4(), Uuid::new_v4(), 4.0, None, vec![]).await;

    // Assert
    assert!(result.is_ok());
}

#[serial_test::serial]
#[tokio::test]
async fn test_confirm_order_received_409_is_ok() {
    // Arrange
    let mock_server = MockServer::start().await;
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", mock_server.uri());
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }
    Mock::given(method("POST"))
        .and(path_regex(r"/internal/products/.+/post-order"))
        .respond_with(ResponseTemplate::new(409).set_body_json(json!({})))
        .mount(&mock_server)
        .await;

    // Act
    let result = confirm_order_received(Uuid::new_v4(), Uuid::new_v4()).await;

    // Assert
    assert!(result.is_ok());
}

#[serial_test::serial]
#[tokio::test]
async fn test_fetch_product_network_error_returns_internal() {
    // Arrange: use unreachable address
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", "http://127.0.0.1:1");
    }
    let result = fetch_product(Uuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::Internal)));
}

#[serial_test::serial]
#[tokio::test]
async fn test_fetch_product_500_returns_internal() {
    let mock_server = MockServer::start().await;
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", mock_server.uri());
    }
    Mock::given(method("GET"))
        .and(path_regex(r"/products/.+"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({})))
        .mount(&mock_server)
        .await;
    let result = fetch_product(Uuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::Internal)));
}

#[serial_test::serial]
#[tokio::test]
async fn test_send_product_rating_500_logs_and_returns_ok() {
    let mock_server = MockServer::start().await;
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", mock_server.uri());
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }
    Mock::given(method("POST"))
        .and(path_regex(r"/internal/products/.+/post-order"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({})))
        .mount(&mock_server)
        .await;
    let result = send_product_rating(Uuid::new_v4(), Uuid::new_v4(), 4.0, None, vec![]).await;
    assert!(result.is_ok());
}

#[serial_test::serial]
#[tokio::test]
async fn test_confirm_order_received_404_returns_not_found() {
    let mock_server = MockServer::start().await;
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", mock_server.uri());
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }
    Mock::given(method("POST"))
        .and(path_regex(r"/internal/products/.+/post-order"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({})))
        .mount(&mock_server)
        .await;
    let result = confirm_order_received(Uuid::new_v4(), Uuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[serial_test::serial]
#[tokio::test]
async fn test_confirm_order_received_500_returns_internal() {
    let mock_server = MockServer::start().await;
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", mock_server.uri());
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }
    Mock::given(method("POST"))
        .and(path_regex(r"/internal/products/.+/post-order"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({})))
        .mount(&mock_server)
        .await;
    let result = confirm_order_received(Uuid::new_v4(), Uuid::new_v4()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn http_inventory_client_adapter_send_product_rating_success() {
    let mock_server = MockServer::start().await;
    let uri = mock_server.uri();

    Mock::given(method("POST"))
        .and(path_regex(r"/internal/products/.+/post-order"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&mock_server)
        .await;

    temp_env::async_with_vars(
        [
            ("INVENTORY_SERVICE_URL", Some(uri.as_str())),
            ("INTERNAL_SERVICE_KEY", Some("test-key")),
        ],
        async {
            let client =
                crate::services::adapters::inventory_client_adapt::HttpInventoryClient;
            let result = client
                .send_product_rating(Uuid::new_v4(), Uuid::new_v4(), 4.5, None, vec![])
                .await;
            assert!(result.is_ok());
        },
    )
    .await;
}
