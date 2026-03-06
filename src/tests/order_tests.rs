use serde_json::json;
use serial_test::serial;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn setup_mocks() -> (MockServer, MockServer) {
    let inventory_server = MockServer::start().await;
    let wallet_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_regex("/products/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "product_id": "550e8400-e29b-41d4-a716-446655440000",
                "jastiper_id": "660e8400-e29b-41d4-a716-446655440000",
                "name": "Tas Korea",
                "description": "Tas import dari Korea",
                "price": 100000,
                "service_fee": 10000,
                "stock": 10,
                "status": "ACTIVE",
                "images": ["https://example.com/image.jpg"],
                "origin_country": "Korea Selatan",
                "purchase_date": "2026-01-01"
            }
        })))
        .mount(&inventory_server)
        .await;

    Mock::given(method("POST"))
        .and(path_regex("/internal/products/.*/stock/reserve"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "product_id": "550e8400-e29b-41d4-a716-446655440000",
            "reserved_quantity": 1,
            "remaining_stock": 9,
            "reservation_id": "770e8400-e29b-41d4-a716-446655440000"
        })))
        .mount(&inventory_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/internal/wallets/deduct"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "transaction_id": "880e8400-e29b-41d4-a716-446655440000",
            "type": "PAYMENT",
            "status": "SUCCESS",
            "new_balance": 500000
        })))
        .mount(&wallet_server)
        .await;

    (inventory_server, wallet_server)
}

#[tokio::test]
#[serial]
async fn test_checkout_success() {
    dotenvy::dotenv().ok();

    let (inventory_server, wallet_server) = setup_mocks().await;

    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", inventory_server.uri());
        std::env::set_var("WALLET_SERVICE_URL", wallet_server.uri());
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL harus diset");
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();

    let req = json!({
        "product_id": "550e8400-e29b-41d4-a716-446655440000",
        "quantity": 1,
        "shipping_address": {
            "recipient_name": "Budi Santoso",
            "phone_number": "081234567890",
            "street": "Jl. Sudirman No. 123",
            "kelurahan": "Senayan",
            "kecamatan": "Kebayoran Baru",
            "city": "Jakarta Selatan",
            "province": "DKI Jakarta",
            "postal_code": "12190",
            "notes": null
        },
        "note_to_jastiper": null
    });

    let pool = std::sync::Arc::new(pool);
    let create_req: crate::models::order::CreateOrderRequest = serde_json::from_value(req).unwrap();

    let product_snapshot = json!({
        "product_id": "550e8400-e29b-41d4-a716-446655440000",
        "name": "Tas Korea",
        "description": "Tas import dari Korea",
        "image_url": "https://example.com/image.jpg",
        "origin_country": "Korea Selatan",
        "purchase_date": "2026-01-01",
        "unit_price": 100000,
        "service_fee": 10000,
    });

    let order = crate::repositories::order::create(
        &pool,
        uuid::Uuid::new_v4(), // titipers_id
        uuid::Uuid::new_v4(), // jastiper_id
        uuid::Uuid::new_v4(), // order_id
        create_req,
        product_snapshot,
        100000i64,
        10000i64,
        110000i64,
    )
    .await;

    assert!(
        order.is_ok(),
        "Order harus berhasil dibuat: {:?}",
        order.err()
    );
    let order = order.unwrap();
    assert_eq!(order.quantity, 1);
    assert_eq!(order.unit_price, 100000i64);

    sqlx::query!(r#"DELETE FROM "order" WHERE order_id = $1"#, order.order_id)
        .execute(&*pool)
        .await
        .unwrap();
}

#[tokio::test]
#[serial]
async fn test_checkout_stok_habis() {
    let inventory_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path_regex("/internal/products/.*/stock/reserve"))
        .respond_with(ResponseTemplate::new(409).set_body_json(json!({
            "message": "Insufficient stock",
            "available_stock": 0,
            "requested": 1
        })))
        .mount(&inventory_server)
        .await;

    Mock::given(method("GET"))
        .and(path_regex("/products/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "product_id": "550e8400-e29b-41d4-a716-446655440000",
                "jastiper_id": "660e8400-e29b-41d4-a716-446655440000",
                "name": "Tas Korea",
                "price": 100000,
                "service_fee": 10000,
                "stock": 0,
                "status": "ACTIVE",
                "images": [],
                "origin_country": "Korea",
                "purchase_date": "2026-01-01"
            }
        })))
        .mount(&inventory_server)
        .await;

    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", inventory_server.uri());
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }

    let result =
        crate::handlers::order::reserve_stock(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), 1).await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::Conflict(_)
    ));
}

#[tokio::test]
#[serial]
async fn test_checkout_saldo_tidak_cukup() {
    dotenvy::dotenv().ok();

    let inventory_server = MockServer::start().await;
    let wallet_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_regex("/products/.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "product_id": "550e8400-e29b-41d4-a716-446655440000",
                "jastiper_id": "660e8400-e29b-41d4-a716-446655440000",
                "name": "Tas Korea",
                "price": 100000,
                "service_fee": 10000,
                "stock": 10,
                "status": "ACTIVE",
                "images": ["https://example.com/image.jpg"],
                "origin_country": "Korea Selatan",
                "purchase_date": "2026-01-01"
            }
        })))
        .mount(&inventory_server)
        .await;

    Mock::given(method("POST"))
        .and(path_regex("/internal/products/.*/stock/reserve"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "reserved_quantity": 1,
            "remaining_stock": 9,
        })))
        .mount(&inventory_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/internal/wallets/deduct"))
        .respond_with(ResponseTemplate::new(422).set_body_json(json!({
            "message": "Insufficient balance",
            "balance": 0,
            "required": 110000
        })))
        .mount(&wallet_server)
        .await;

    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", inventory_server.uri());
        std::env::set_var("WALLET_SERVICE_URL", wallet_server.uri());
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }

    let result = crate::handlers::order::deduct_wallet(
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        110000i64,
        "Pembayaran test",
    )
    .await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::UnprocessableEntity(_)
    ));
}
