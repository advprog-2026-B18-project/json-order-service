use serde_json::json;
use serial_test::serial;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS — Pool
// ═══════════════════════════════════════════════════════════════════════════════

async fn make_pool() -> sqlx::PgPool {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL harus diset");
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(&database_url)
        .await
        .expect("Gagal koneksi ke DB")
}

async fn make_arc_pool() -> std::sync::Arc<sqlx::PgPool> {
    std::sync::Arc::new(make_pool().await)
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS — Fixture JSON
// ═══════════════════════════════════════════════════════════════════════════════

fn shipping_address_json() -> serde_json::Value {
    json!({
        "recipient_name": "Budi Santoso",
        "phone_number": "081234567890",
        "street": "Jl. Sudirman No. 123",
        "kelurahan": "Senayan",
        "kecamatan": "Kebayoran Baru",
        "city": "Jakarta Selatan",
        "province": "DKI Jakarta",
        "postal_code": "12190",
        "notes": null
    })
}

fn product_snapshot_json() -> serde_json::Value {
    json!({
        "product_id": "550e8400-e29b-41d4-a716-446655440000",
        "name": "Tas Korea",
        "description": "Tas import dari Korea",
        "image_url": "https://example.com/image.jpg",
        "origin_country": "Korea Selatan",
        "purchase_date": "2026-01-01",
        "unit_price": 100000,
        "service_fee": 10000,
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS — Mock Servers
// ═══════════════════════════════════════════════════════════════════════════════

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

/// Setup mock wallet dengan body JSON.
async fn mock_wallet_with_body(path_str: &str, status: u16, body: serde_json::Value) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(path_str))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .mount(&server)
        .await;
    server
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS — Order fixture & cleanup
// ═══════════════════════════════════════════════════════════════════════════════

async fn create_test_order(pool: &std::sync::Arc<sqlx::PgPool>) -> crate::models::order::Order {
    let req: crate::models::order::CreateOrderRequest = serde_json::from_value(json!({
        "product_id": "550e8400-e29b-41d4-a716-446655440000",
        "quantity": 1,
        "shipping_address": shipping_address_json(),
        "note_to_jastiper": null
    }))
    .unwrap();

    crate::repositories::order::create(
        pool,
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        req,
        product_snapshot_json(),
        100000i64,
        10000i64,
        110000i64,
    )
    .await
    .expect("Gagal membuat order untuk test")
}

async fn cleanup_order(pool: &sqlx::PgPool, order_id: uuid::Uuid) {
    sqlx::query(r#"DELETE FROM order_status_history WHERE order_id = $1"#)
        .bind(order_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query(r#"DELETE FROM "order" WHERE order_id = $1"#)
        .bind(order_id)
        .execute(pool)
        .await
        .ok();
}

/// Helper untuk set env vars yang digunakan oleh inventory + wallet handler.
fn set_service_envs(inventory_uri: &str, wallet_uri: &str) {
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", inventory_uri);
        std::env::set_var("WALLET_SERVICE_URL", wallet_uri);
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }
}

fn set_inventory_env(uri: &str) {
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", uri);
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }
}

fn set_wallet_env(uri: &str) {
    unsafe {
        std::env::set_var("WALLET_SERVICE_URL", uri);
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// REPOSITORIES — order.rs
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[serial]
async fn test_repo_create_dan_find_by_id() {
    let pool = make_arc_pool().await;
    let order = create_test_order(&pool).await;

    assert_eq!(order.quantity, 1);
    assert_eq!(order.unit_price, 100000i64);
    assert_eq!(order.service_fee, 10000i64);
    assert_eq!(order.total_price, 110000i64);

    let found = crate::repositories::order::find_by_id(&pool, order.order_id)
        .await
        .expect("find_by_id gagal");
    assert!(found.is_some());
    assert_eq!(found.unwrap().order_id, order.order_id);

    cleanup_order(&pool, order.order_id).await;
}

#[tokio::test]
#[serial]
async fn test_repo_find_by_id_tidak_ada() {
    let pool = make_pool().await;
    let result = crate::repositories::order::find_by_id(&pool, uuid::Uuid::new_v4())
        .await
        .expect("Query tidak boleh error");
    assert!(result.is_none());
}

#[tokio::test]
#[serial]
async fn test_repo_find_all_dengan_filter_titipers() {
    let pool = make_arc_pool().await;
    let order = create_test_order(&pool).await;

    let filter = Some(crate::models::order::OrderFilter {
        titipers_id: Some(order.titipers_id),
        ..Default::default()
    });
    let (orders, total) = crate::repositories::order::find_all(&pool, filter, Some(1), Some(20))
        .await
        .expect("find_all gagal");

    assert!(total >= 1);
    assert!(orders.iter().any(|o| o.order_id == order.order_id));

    cleanup_order(&pool, order.order_id).await;
}

#[tokio::test]
#[serial]
async fn test_repo_find_all_dengan_filter_jastiper() {
    let pool = make_arc_pool().await;
    let order = create_test_order(&pool).await;

    let filter = Some(crate::models::order::OrderFilter {
        jastiper_id: Some(order.jastiper_id),
        ..Default::default()
    });
    let (orders, total) = crate::repositories::order::find_all(&pool, filter, None, None)
        .await
        .expect("find_all gagal");

    assert!(total >= 1);
    assert!(orders.iter().any(|o| o.order_id == order.order_id));

    cleanup_order(&pool, order.order_id).await;
}

#[tokio::test]
#[serial]
async fn test_repo_find_all_tanpa_filter() {
    let pool = make_arc_pool().await;
    let order = create_test_order(&pool).await;

    let (orders, total) = crate::repositories::order::find_all(&pool, None, Some(1), Some(10))
        .await
        .expect("find_all tanpa filter gagal");

    assert!(total >= 1);
    assert!(!orders.is_empty());

    cleanup_order(&pool, order.order_id).await;
}

#[tokio::test]
#[serial]
async fn test_repo_insert_status_history() {
    let pool = make_arc_pool().await;
    let order = create_test_order(&pool).await;

    // insert_status_history dipanggil saat create, cek hasilnya
    let history = crate::repositories::order::get_status_history(&pool, order.order_id)
        .await
        .expect("get_status_history gagal");

    assert!(!history.is_empty());
    assert_eq!(history[0].status, "PAID");
    assert_eq!(history[0].actor_role, "TITIPERS");

    cleanup_order(&pool, order.order_id).await;
}

#[tokio::test]
#[serial]
async fn test_repo_update_status_valid_transition() {
    let pool = make_arc_pool().await;
    let order = create_test_order(&pool).await;

    let updated = crate::repositories::order::update_status(
        &pool,
        order.order_id,
        &crate::models::order::OrderStatus::Purchased,
        "jastiper-uuid",
        "JASTIPER",
        Some("Barang sudah dibeli"),
        None,
        None,
    )
    .await
    .expect("update_status gagal");

    assert_eq!(updated.status, crate::models::order::OrderStatus::Purchased);

    let history = crate::repositories::order::get_status_history(&pool, order.order_id)
        .await
        .unwrap();
    assert!(history.iter().any(|h| h.status == "PURCHASED"));

    cleanup_order(&pool, order.order_id).await;
}

#[tokio::test]
#[serial]
async fn test_repo_update_status_dengan_tracking() {
    let pool = make_arc_pool().await;
    let order = create_test_order(&pool).await;

    // PAID → PURCHASED
    crate::repositories::order::update_status(
        &pool,
        order.order_id,
        &crate::models::order::OrderStatus::Purchased,
        "jastiper-uuid",
        "JASTIPER",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // PURCHASED → SHIPPED dengan tracking
    let updated = crate::repositories::order::update_status(
        &pool,
        order.order_id,
        &crate::models::order::OrderStatus::Shipped,
        "jastiper-uuid",
        "JASTIPER",
        Some("Paket dikirim"),
        Some("JNE-12345"),
        Some("JNE"),
    )
    .await
    .expect("update ke SHIPPED gagal");

    assert_eq!(updated.status, crate::models::order::OrderStatus::Shipped);
    assert_eq!(updated.tracking_number.as_deref(), Some("JNE-12345"));
    assert_eq!(updated.courier.as_deref(), Some("JNE"));

    cleanup_order(&pool, order.order_id).await;
}

#[tokio::test]
#[serial]
async fn test_repo_update_status_ke_completed_set_completed_at() {
    let pool = make_arc_pool().await;
    let order = create_test_order(&pool).await;

    for (status, role) in [
        (crate::models::order::OrderStatus::Purchased, "JASTIPER"),
        (crate::models::order::OrderStatus::Shipped, "JASTIPER"),
    ] {
        crate::repositories::order::update_status(
            &pool,
            order.order_id,
            &status,
            "actor",
            role,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    }

    let completed = crate::repositories::order::update_status(
        &pool,
        order.order_id,
        &crate::models::order::OrderStatus::Completed,
        "titipers-uuid",
        "TITIPERS",
        Some("Barang diterima"),
        None,
        None,
    )
    .await
    .expect("update ke COMPLETED gagal");

    assert_eq!(
        completed.status,
        crate::models::order::OrderStatus::Completed
    );
    assert!(completed.completed_at.is_some());

    cleanup_order(&pool, order.order_id).await;
}

#[tokio::test]
#[serial]
async fn test_repo_update_status_invalid_transition() {
    let pool = make_arc_pool().await;
    let order = create_test_order(&pool).await; // status PAID

    // PAID → COMPLETED langsung tidak valid
    let result = crate::repositories::order::update_status(
        &pool,
        order.order_id,
        &crate::models::order::OrderStatus::Completed,
        "actor",
        "ADMIN",
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::InvalidStatusTransition { .. }
    ));

    cleanup_order(&pool, order.order_id).await;
}

#[tokio::test]
#[serial]
async fn test_repo_update_status_order_tidak_ada() {
    let pool = make_pool().await;

    let result = crate::repositories::order::update_status(
        &pool,
        uuid::Uuid::new_v4(),
        &crate::models::order::OrderStatus::Purchased,
        "actor",
        "JASTIPER",
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::NotFound(_)
    ));
}

#[tokio::test]
#[serial]
async fn test_repo_cancel_order() {
    let pool = make_arc_pool().await;
    let order = create_test_order(&pool).await;

    let cancelled = crate::repositories::order::cancel_order(
        &pool,
        order.order_id,
        "OTHER",
        &crate::models::order::CancelledBy::Jastiper,
        "jastiper-uuid",
        "JASTIPER",
        Some("Dibatalkan oleh jastiper"),
    )
    .await
    .expect("cancel_order gagal");

    assert_eq!(
        cancelled.status,
        crate::models::order::OrderStatus::Cancelled
    );
    assert_eq!(cancelled.cancellation_reason.as_deref(), Some("OTHER"));

    let history = crate::repositories::order::get_status_history(&pool, order.order_id)
        .await
        .unwrap();
    assert!(history.iter().any(|h| h.status == "CANCELLED"));

    cleanup_order(&pool, order.order_id).await;
}

#[tokio::test]
#[serial]
async fn test_repo_cancel_order_tidak_ada() {
    let pool = make_pool().await;

    let result = crate::repositories::order::cancel_order(
        &pool,
        uuid::Uuid::new_v4(),
        "Alasan",
        &crate::models::order::CancelledBy::Admin,
        "admin-uuid",
        "ADMIN",
        None,
    )
    .await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::NotFound(_)
    ));
}

#[tokio::test]
#[serial]
async fn test_repo_cancel_order_dari_terminal_state() {
    let pool = make_arc_pool().await;
    let order = create_test_order(&pool).await;

    // Cancel pertama → berhasil
    crate::repositories::order::cancel_order(
        &pool,
        order.order_id,
        "TRIP_CANCELLED",
        &crate::models::order::CancelledBy::Admin,
        "admin",
        "ADMIN",
        None,
    )
    .await
    .unwrap();

    // Cancel kedua dari CANCELLED → harus gagal (terminal state)
    let result = crate::repositories::order::cancel_order(
        &pool,
        order.order_id,
        "OTHER",
        &crate::models::order::CancelledBy::Admin,
        "admin",
        "ADMIN",
        None,
    )
    .await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::InvalidStatusTransition { .. }
    ));

    cleanup_order(&pool, order.order_id).await;
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLERS — checkout
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[serial]
async fn test_checkout_success() {
    let (inventory_server, wallet_server) = setup_mocks().await;
    set_service_envs(&inventory_server.uri(), &wallet_server.uri());

    let pool = make_arc_pool().await;
    let req: crate::models::order::CreateOrderRequest = serde_json::from_value(json!({
        "product_id": "550e8400-e29b-41d4-a716-446655440000",
        "quantity": 1,
        "shipping_address": shipping_address_json(),
        "note_to_jastiper": null
    }))
    .unwrap();

    let order = crate::repositories::order::create(
        &pool,
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        req,
        product_snapshot_json(),
        100000i64,
        10000i64,
        110000i64,
    )
    .await;

    assert!(order.is_ok(), "Order harus berhasil: {:?}", order.err());
    let order = order.unwrap();
    assert_eq!(order.quantity, 1);
    assert_eq!(order.unit_price, 100000i64);

    cleanup_order(&pool, order.order_id).await;
}

#[tokio::test]
#[serial]
async fn test_checkout_stok_habis() {
    let inventory_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex("/internal/products/.*/stock/reserve"))
        .respond_with(
            ResponseTemplate::new(409).set_body_json(json!({"message": "Insufficient stock"})),
        )
        .mount(&inventory_server)
        .await;
    set_inventory_env(&inventory_server.uri());

    let result =
        crate::handlers::order::reserve_stock(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), 1).await;

    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::Conflict(_)
    ));
}

#[tokio::test]
#[serial]
async fn test_checkout_saldo_tidak_cukup() {
    let wallet_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/wallets/deduct"))
        .respond_with(
            ResponseTemplate::new(422).set_body_json(json!({"message": "Insufficient balance"})),
        )
        .mount(&wallet_server)
        .await;
    set_wallet_env(&wallet_server.uri());

    let result = crate::handlers::order::deduct_wallet(
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        110000i64,
        "Pembayaran test",
    )
    .await;

    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::UnprocessableEntity(_)
    ));
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLERS — reserve_stock
// ═══════════════════════════════════════════════════════════════════════════════

async fn setup_reserve_mock(status: u16) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex("/internal/products/.*/stock/reserve"))
        .respond_with(ResponseTemplate::new(status))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
#[serial]
async fn test_reserve_stock_produk_tidak_ditemukan() {
    let server = setup_reserve_mock(404).await;
    set_inventory_env(&server.uri());

    let result =
        crate::handlers::order::reserve_stock(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), 1).await;

    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::NotFound(_)
    ));
}

#[tokio::test]
#[serial]
async fn test_reserve_stock_produk_tidak_aktif() {
    let server = setup_reserve_mock(422).await;
    set_inventory_env(&server.uri());

    let result =
        crate::handlers::order::reserve_stock(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), 1).await;

    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::UnprocessableEntity(_)
    ));
}

#[tokio::test]
#[serial]
async fn test_reserve_stock_server_error() {
    let server = setup_reserve_mock(500).await;
    set_inventory_env(&server.uri());

    let result =
        crate::handlers::order::reserve_stock(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), 1).await;

    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::Internal
    ));
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLERS — release_stock
// ═══════════════════════════════════════════════════════════════════════════════

async fn setup_release_mock(status: u16) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex("/internal/products/.*/stock/release"))
        .respond_with(ResponseTemplate::new(status))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
#[serial]
async fn test_release_stock_success() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path_regex("/internal/products/.*/stock/release"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "ok"})))
        .mount(&server)
        .await;
    set_inventory_env(&server.uri());

    let result =
        crate::handlers::order::release_stock(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), 1).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_release_stock_tidak_ditemukan() {
    let server = setup_release_mock(404).await;
    set_inventory_env(&server.uri());

    let result =
        crate::handlers::order::release_stock(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), 1).await;
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::NotFound(_)
    ));
}

#[tokio::test]
#[serial]
async fn test_release_stock_conflict() {
    let server = setup_release_mock(409).await;
    set_inventory_env(&server.uri());

    let result =
        crate::handlers::order::release_stock(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), 1).await;
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::Conflict(_)
    ));
}

#[tokio::test]
#[serial]
async fn test_release_stock_unprocessable() {
    let server = setup_release_mock(422).await;
    set_inventory_env(&server.uri());

    let result =
        crate::handlers::order::release_stock(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), 1).await;
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::UnprocessableEntity(_)
    ));
}

#[tokio::test]
#[serial]
async fn test_release_stock_server_error() {
    let server = setup_release_mock(500).await;
    set_inventory_env(&server.uri());

    let result =
        crate::handlers::order::release_stock(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), 1).await;
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::Internal
    ));
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLERS — deduct_wallet
// ═══════════════════════════════════════════════════════════════════════════════

async fn setup_deduct_mock(status: u16) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/wallets/deduct"))
        .respond_with(ResponseTemplate::new(status))
        .mount(&server)
        .await;
    server
}

async fn call_deduct_wallet(uri: &str) -> Result<(), crate::error::AppError> {
    set_wallet_env(uri);
    crate::handlers::order::deduct_wallet(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), 50000, "Test")
        .await
}

#[tokio::test]
#[serial]
async fn test_deduct_wallet_success() {
    let server =
        mock_wallet_with_body("/internal/wallets/deduct", 200, json!({"status": "ok"})).await;
    let result = call_deduct_wallet(&server.uri()).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_deduct_wallet_user_tidak_ditemukan() {
    let server = setup_deduct_mock(404).await;
    let result = call_deduct_wallet(&server.uri()).await;
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::NotFound(_)
    ));
}

#[tokio::test]
#[serial]
async fn test_deduct_wallet_idempotent_409() {
    let server = setup_deduct_mock(409).await;
    // 409 harus dianggap sukses (idempotent)
    let result = call_deduct_wallet(&server.uri()).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_deduct_wallet_server_error() {
    let server = setup_deduct_mock(500).await;
    let result = call_deduct_wallet(&server.uri()).await;
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::Internal
    ));
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLERS — refund_wallet
// ═══════════════════════════════════════════════════════════════════════════════

async fn setup_refund_mock(status: u16) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/wallets/refund"))
        .respond_with(ResponseTemplate::new(status))
        .mount(&server)
        .await;
    server
}

async fn call_refund_wallet(uri: &str) -> Result<(), crate::error::AppError> {
    set_wallet_env(uri);
    crate::handlers::order::refund_wallet(
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        50000,
        "Refund test",
    )
    .await
}

#[tokio::test]
#[serial]
async fn test_refund_wallet_success() {
    let server =
        mock_wallet_with_body("/internal/wallets/refund", 200, json!({"status": "ok"})).await;
    let result = call_refund_wallet(&server.uri()).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_refund_wallet_idempotent_409() {
    let server = setup_refund_mock(409).await;
    // 409 harus dianggap sukses (idempotent)
    let result = call_refund_wallet(&server.uri()).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[serial]
async fn test_refund_wallet_server_error() {
    let server = setup_refund_mock(500).await;
    let result = call_refund_wallet(&server.uri()).await;
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::Internal
    ));
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLERS — fetch_product
// ═══════════════════════════════════════════════════════════════════════════════

async fn setup_fetch_product_mock(status: u16) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex("/products/.*"))
        .respond_with(ResponseTemplate::new(status))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
#[serial]
async fn test_fetch_product_success() {
    let inventory_server = MockServer::start().await;
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
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", inventory_server.uri());
    }

    let result = crate::handlers::order::fetch_product(uuid::Uuid::new_v4()).await;
    assert!(result.is_ok());
    let data = result.unwrap();
    assert_eq!(data["name"], "Tas Korea");
}

#[tokio::test]
#[serial]
async fn test_fetch_product_tidak_ditemukan() {
    let server = setup_fetch_product_mock(404).await;
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", server.uri());
    }

    let result = crate::handlers::order::fetch_product(uuid::Uuid::new_v4()).await;
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::NotFound(_)
    ));
}

#[tokio::test]
#[serial]
async fn test_fetch_product_tidak_aktif() {
    let server = setup_fetch_product_mock(422).await;
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", server.uri());
    }

    let result = crate::handlers::order::fetch_product(uuid::Uuid::new_v4()).await;
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::UnprocessableEntity(_)
    ));
}

#[tokio::test]
#[serial]
async fn test_fetch_product_server_error() {
    let server = setup_fetch_product_mock(500).await;
    unsafe {
        std::env::set_var("INVENTORY_SERVICE_URL", server.uri());
    }

    let result = crate::handlers::order::fetch_product(uuid::Uuid::new_v4()).await;
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::Internal
    ));
}

// ═══════════════════════════════════════════════════════════════════════════════
// MODELS — order.rs: state machine
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_state_machine_valid_transitions() {
    use crate::models::order::OrderStatus::*;

    assert!(Pending.can_transition_to(&Paid));
    assert!(Pending.can_transition_to(&Cancelled));
    assert!(Paid.can_transition_to(&Purchased));
    assert!(Paid.can_transition_to(&Cancelled));
    assert!(Purchased.can_transition_to(&Shipped));
    assert!(Purchased.can_transition_to(&Cancelled));
    assert!(Shipped.can_transition_to(&Completed));
    assert!(Shipped.can_transition_to(&Cancelled));
}

#[test]
fn test_state_machine_invalid_transitions() {
    use crate::models::order::OrderStatus::*;

    assert!(!Pending.can_transition_to(&Purchased));
    assert!(!Paid.can_transition_to(&Completed));
    assert!(!Shipped.can_transition_to(&Pending));
    assert!(!Completed.can_transition_to(&Cancelled)); // terminal
    assert!(!Cancelled.can_transition_to(&Paid)); // terminal
}

#[test]
fn test_state_machine_terminal_states_kosong() {
    use crate::models::order::OrderStatus::*;

    assert!(Completed.valid_next().is_empty());
    assert!(Cancelled.valid_next().is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════════
// ERROR — error.rs: IntoResponse coverage
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_error_into_response_semua_variant() {
    use crate::error::AppError;
    use axum::response::IntoResponse;

    let cases: Vec<(AppError, u16)> = vec![
        (AppError::Validation("invalid".to_string()), 400),
        (AppError::Unauthorized("unauth".to_string()), 401),
        (AppError::Forbidden("forbidden".to_string()), 403),
        (AppError::NotFound("not found".to_string()), 404),
        (AppError::Conflict("conflict".to_string()), 409),
        (
            AppError::UnprocessableEntity("unprocessable".to_string()),
            422,
        ),
        (
            AppError::InvalidStatusTransition {
                current: "PAID".to_string(),
                requested: "COMPLETED".to_string(),
                valid: vec!["PURCHASED".to_string()],
            },
            422,
        ),
        (AppError::LimitExceeded, 400),
        (AppError::Internal, 500),
    ];

    for (err, expected_status) in cases {
        let response = err.into_response();
        assert_eq!(
            response.status().as_u16(),
            expected_status,
            "Status code tidak sesuai"
        );
    }
}

#[test]
fn test_error_database_into_response() {
    use crate::error::AppError;
    use axum::response::IntoResponse;

    let sqlx_err = sqlx::Error::RowNotFound;
    let err = AppError::Database(sqlx_err);
    let response = err.into_response();
    assert_eq!(response.status().as_u16(), 500);
}

// ═══════════════════════════════════════════════════════════════════════════════
// MIDDLEWARE — auth.rs
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_jwt_claims_user_id_valid() {
    use crate::middleware::auth::JwtClaims;

    let valid_uuid = uuid::Uuid::new_v4().to_string();
    let claims = JwtClaims {
        sub: valid_uuid.clone(),
        email: "test@example.com".to_string(),
        role: "TITIPERS".to_string(),
        exp: 9999999999,
        iat: 0,
    };

    let result = claims.user_id();
    assert!(result.is_ok());
    assert_eq!(result.unwrap().to_string(), valid_uuid);
}

#[test]
fn test_jwt_claims_user_id_invalid_uuid() {
    use crate::middleware::auth::JwtClaims;

    let claims = JwtClaims {
        sub: "bukan-uuid".to_string(),
        email: "test@example.com".to_string(),
        role: "TITIPERS".to_string(),
        exp: 9999999999,
        iat: 0,
    };

    let result = claims.user_id();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::Unauthorized(_)
    ));
}

/// Helper: buat request parts dengan header Authorization opsional.
fn make_request_parts(auth_header: Option<&'static str>) -> axum::http::request::Parts {
    let mut builder = axum::http::Request::builder();
    if let Some(value) = auth_header {
        builder = builder.header(axum::http::header::AUTHORIZATION, value);
    }
    let (parts, _) = builder.body(()).unwrap().into_parts();
    parts
}

#[tokio::test]
async fn test_jwt_from_request_parts_tanpa_header() {
    use crate::middleware::auth::JwtClaims;
    use axum::extract::FromRequestParts;

    let mut parts = make_request_parts(None);
    let result = JwtClaims::from_request_parts(&mut parts, &()).await;
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::Unauthorized(_)
    ));
}

#[tokio::test]
async fn test_jwt_from_request_parts_format_salah() {
    use crate::middleware::auth::JwtClaims;
    use axum::extract::FromRequestParts;

    let mut parts = make_request_parts(Some("Token abc123"));
    let result = JwtClaims::from_request_parts(&mut parts, &()).await;
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::Unauthorized(_)
    ));
}

#[tokio::test]
async fn test_jwt_from_request_parts_token_invalid() {
    use crate::middleware::auth::JwtClaims;
    use axum::extract::FromRequestParts;

    unsafe {
        std::env::set_var("JWT_SECRET", "test-secret");
    }

    let mut parts = make_request_parts(Some("Bearer token.tidak.valid"));
    let result = JwtClaims::from_request_parts(&mut parts, &()).await;
    assert!(matches!(
        result.unwrap_err(),
        crate::error::AppError::Unauthorized(_)
    ));
}

#[tokio::test]
async fn test_jwt_from_request_parts_token_valid() {
    use crate::middleware::auth::JwtClaims;
    use axum::extract::FromRequestParts;
    use jsonwebtoken::{EncodingKey, Header, encode};

    let secret = "test-secret-jwt";
    unsafe {
        std::env::set_var("JWT_SECRET", secret);
    }

    let user_id = uuid::Uuid::new_v4();
    let claims = JwtClaims {
        sub: user_id.to_string(),
        email: "user@example.com".to_string(),
        role: "TITIPERS".to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        iat: chrono::Utc::now().timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("Gagal encode JWT");

    let bearer = format!("Bearer {}", token);
    // Tidak bisa pakai make_request_parts karena value bukan &'static str
    let (mut parts, _) = axum::http::Request::builder()
        .header(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_str(&bearer).unwrap(),
        )
        .body(())
        .unwrap()
        .into_parts();

    let result = JwtClaims::from_request_parts(&mut parts, &()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().sub, user_id.to_string());
}
