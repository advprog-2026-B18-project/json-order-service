use mockito::Server;
use serde_json::json;
use uuid::Uuid;

fn setup() {
    unsafe {
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }
}

#[tokio::test]
#[serial_test::serial]
async fn reserve_stock_sukses() {
    setup();
    let mut server = Server::new_async().await;
    let product_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", server.url());
    }

    server
        .mock(
            "POST",
            format!("/products/internal/{}/stock/reserve", product_id).as_str(),
        )
        .with_status(200)
        .create_async()
        .await;

    let result =
        crate::services::inventory_client::reserve_stock(product_id, Uuid::new_v4(), 2).await;

    assert!(result.is_ok());
}

#[tokio::test]
#[serial_test::serial]
async fn reserve_stock_gagal_produk_tidak_ditemukan_404() {
    setup();
    let mut server = Server::new_async().await;
    let product_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", server.url());
    }

    server
        .mock(
            "POST",
            format!("/products/internal/{}/stock/reserve", product_id).as_str(),
        )
        .with_status(404)
        .create_async()
        .await;

    let result =
        crate::services::inventory_client::reserve_stock(product_id, Uuid::new_v4(), 1).await;

    assert!(matches!(result, Err(crate::error::AppError::NotFound(_))));
}

#[tokio::test]
#[serial_test::serial]
async fn reserve_stock_gagal_stok_tidak_cukup_409() {
    setup();
    let mut server = Server::new_async().await;
    let product_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", server.url());
    }

    server
        .mock(
            "POST",
            format!("/products/internal/{}/stock/reserve", product_id).as_str(),
        )
        .with_status(409)
        .create_async()
        .await;

    let result =
        crate::services::inventory_client::reserve_stock(product_id, Uuid::new_v4(), 100).await;

    assert!(matches!(result, Err(crate::error::AppError::Conflict(_))));
}

#[tokio::test]
#[serial_test::serial]
async fn reserve_stock_gagal_produk_tidak_aktif_422() {
    setup();
    let mut server = Server::new_async().await;
    let product_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", server.url());
    }

    server
        .mock(
            "POST",
            format!("/products/internal/{}/stock/reserve", product_id).as_str(),
        )
        .with_status(422)
        .create_async()
        .await;

    let result =
        crate::services::inventory_client::reserve_stock(product_id, Uuid::new_v4(), 1).await;

    assert!(matches!(
        result,
        Err(crate::error::AppError::UnprocessableEntity(_))
    ));
}

#[tokio::test]
#[serial_test::serial]
async fn release_stock_sukses() {
    setup();
    let mut server = Server::new_async().await;
    let product_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", server.url());
    }

    server
        .mock(
            "POST",
            format!("/products/internal/{}/stock/release", product_id).as_str(),
        )
        .with_status(200)
        .create_async()
        .await;

    let result =
        crate::services::inventory_client::release_stock(product_id, Uuid::new_v4(), 1).await;

    assert!(result.is_ok());
}

#[tokio::test]
#[serial_test::serial]
async fn release_stock_gagal_produk_tidak_ditemukan_404() {
    setup();
    let mut server = Server::new_async().await;
    let product_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", server.url());
    }

    server
        .mock(
            "POST",
            format!("/products/internal/{}/stock/release", product_id).as_str(),
        )
        .with_status(404)
        .create_async()
        .await;

    let result =
        crate::services::inventory_client::release_stock(product_id, Uuid::new_v4(), 1).await;

    assert!(matches!(result, Err(crate::error::AppError::NotFound(_))));
}

#[tokio::test]
#[serial_test::serial]
async fn fetch_product_sukses() {
    setup();
    let mut server = Server::new_async().await;
    let product_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", server.url());
    }

    let product_data = json!({
        "jastiperId": Uuid::new_v4(),
        "name": "Snickers",
        "price": 10_000_i64,
        "service_fee": 1_000_i64,
    });

    server
        .mock("GET", format!("/products/{}", product_id).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json!({ "data": product_data }).to_string())
        .create_async()
        .await;

    let result = crate::services::inventory_client::fetch_product(product_id).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap()["name"], "Snickers");
}

#[tokio::test]
#[serial_test::serial]
async fn fetch_product_gagal_tidak_ditemukan_404() {
    setup();
    let mut server = Server::new_async().await;
    let product_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", server.url());
    }

    server
        .mock("GET", format!("/products/{}", product_id).as_str())
        .with_status(404)
        .create_async()
        .await;

    let result = crate::services::inventory_client::fetch_product(product_id).await;

    assert!(matches!(result, Err(crate::error::AppError::NotFound(_))));
}

#[tokio::test]
#[serial_test::serial]
async fn fetch_product_gagal_produk_tidak_aktif_422() {
    setup();
    let mut server = Server::new_async().await;
    let product_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", server.url());
    }

    server
        .mock("GET", format!("/products/{}", product_id).as_str())
        .with_status(422)
        .create_async()
        .await;

    let result = crate::services::inventory_client::fetch_product(product_id).await;
    println!("Result: {:?}", result);

    assert!(matches!(
        result,
        Err(crate::error::AppError::UnprocessableEntity(_))
    ));
}

#[tokio::test]
#[serial_test::serial]
async fn fetch_product_gagal_unexpected_status_500() {
    setup();
    let mut server = Server::new_async().await;
    let product_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", server.url());
    }

    server
        .mock("GET", format!("/products/{}", product_id).as_str())
        .with_status(500)
        .create_async()
        .await;

    let result = crate::services::inventory_client::fetch_product(product_id).await;

    assert!(matches!(result, Err(crate::error::AppError::Internal)));
}

#[tokio::test]
#[serial_test::serial]
async fn send_product_rating_sukses() {
    setup();
    let mut server = Server::new_async().await;
    let product_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", server.url());
    }

    server
        .mock(
            "POST",
            format!("/internal/products/{}/post-order", product_id).as_str(),
        )
        .with_status(200)
        .create_async()
        .await;

    let result = crate::services::inventory_client::send_product_rating(
        product_id,
        Uuid::new_v4(),
        4.5,
        Some("Produk bagus"),
        vec!["https://img.example.com/1.jpg"],
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
#[serial_test::serial]
async fn send_product_rating_produk_tidak_ditemukan_404_non_fatal() {
    setup();
    let mut server = Server::new_async().await;
    let product_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", server.url());
    }

    server
        .mock(
            "POST",
            format!("/internal/products/{}/post-order", product_id).as_str(),
        )
        .with_status(404)
        .create_async()
        .await;

    let result = crate::services::inventory_client::send_product_rating(
        product_id,
        Uuid::new_v4(),
        4.0,
        None,
        vec![],
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
#[serial_test::serial]
async fn send_product_rating_idempotent_409() {
    setup();
    let mut server = Server::new_async().await;
    let product_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", server.url());
    }

    server
        .mock(
            "POST",
            format!("/internal/products/{}/post-order", product_id).as_str(),
        )
        .with_status(409)
        .create_async()
        .await;

    let result = crate::services::inventory_client::send_product_rating(
        product_id,
        Uuid::new_v4(),
        5.0,
        Some("Mantap"),
        vec![],
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
#[serial_test::serial]
async fn send_product_rating_unexpected_status_tetap_ok() {
    setup();
    let mut server = Server::new_async().await;
    let product_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", server.url());
    }

    server
        .mock(
            "POST",
            format!("/internal/products/{}/post-order", product_id).as_str(),
        )
        .with_status(503)
        .create_async()
        .await;

    let result = crate::services::inventory_client::send_product_rating(
        product_id,
        Uuid::new_v4(),
        3.0,
        None,
        vec![],
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
#[serial_test::serial]
async fn send_product_rating_tanpa_review_dan_images() {
    setup();
    let mut server = Server::new_async().await;
    let product_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", server.url());
    }

    server
        .mock(
            "POST",
            format!("/internal/products/{}/post-order", product_id).as_str(),
        )
        .with_status(200)
        .create_async()
        .await;

    let result = crate::services::inventory_client::send_product_rating(
        product_id,
        Uuid::new_v4(),
        4.0,
        None,
        vec![],
    )
    .await;

    assert!(result.is_ok());
}
