use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::order::Order;
use crate::models::order_state::OrderStatus;
use crate::orchestrator::SagaStep;
use crate::orchestrator::checkout_saga::{
    CheckWalletStep, CheckoutContext, CreateOrderStep, ReserveStockStep, build_checkout_context,
};
use crate::repositories::order_repository::MockOrderRepository;
use crate::services::inventory_client::MockInventoryClient;
use crate::services::wallet_client::MockWalletClient;

// ──────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────

fn make_order(order_id: Uuid, titipers_id: Uuid, jastiper_id: Uuid) -> Order {
    Order {
        order_id,
        titipers_id,
        jastiper_id,
        product_id: Uuid::new_v4(),
        product_snapshot: json!({}),
        quantity: 2,
        unit_price: 50_000,
        service_fee: 5_000,
        total_price: 110_000,
        status: OrderStatus::Pending,
        shipping_address: json!({}),
        note_to_jastiper: None,
        tracking_number: None,
        courier: None,
        cancellation_reason: None,
        cancelled_by: None,
        completed_at: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

fn make_checkout_ctx(titipers_id: Uuid, jastiper_id: Uuid) -> CheckoutContext {
    use crate::models::order::CreateOrderRequest;
    use crate::models::shipping_address::ShippingAddress;

    let product = json!({
        "jastiper": { "user_id": jastiper_id },
        "name": "Snickers",
        "description": "Coklat",
        "images": ["http://img.url"],
        "originCountry": "Japan",
        "purchaseDate": "2026-01-01",
        "price": 50_000_i64,
        "service_fee": 5_000_i64,
    });

    let req = CreateOrderRequest {
        product_id: Uuid::new_v4(),
        quantity: 2,
        shipping_address: ShippingAddress {
            recipient_name: "Test User".to_string(),
            phone_number: "08123456789".to_string(),
            street: "Jl. Test No.1".to_string(),
            kelurahan: "Kel".to_string(),
            kecamatan: "Kec".to_string(),
            city: "Jakarta".to_string(),
            province: "DKI".to_string(),
            postal_code: "12345".to_string(),
            notes: None,
        },
        note_to_jastiper: None,
    };

    build_checkout_context(titipers_id, jastiper_id, req, product)
}

// ──────────────────────────────────────────────────────────────
// CheckWalletStep
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn check_wallet_step_execute_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let mut wallet = MockWalletClient::new();
    wallet.expect_check_wallet().returning(|_, _| Ok(()));

    let step = CheckWalletStep {
        wallet_client: Arc::new(wallet),
    };

    let mut ctx = make_checkout_ctx(titipers_id, jastiper_id);
    let result = step.execute(&mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn check_wallet_step_execute_gagal_saldo_kurang() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let mut wallet = MockWalletClient::new();
    wallet.expect_check_wallet().returning(|_, _| {
        Err(AppError::UnprocessableEntity(
            "Saldo tidak cukup".to_string(),
        ))
    });

    let step = CheckWalletStep {
        wallet_client: Arc::new(wallet),
    };

    let mut ctx = make_checkout_ctx(titipers_id, jastiper_id);
    let result = step.execute(&mut ctx).await;
    assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
}

#[tokio::test]
async fn check_wallet_step_compensate_noop() {
    let wallet = MockWalletClient::new();
    let step = CheckWalletStep {
        wallet_client: Arc::new(wallet),
    };

    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let mut ctx = make_checkout_ctx(titipers_id, jastiper_id);
    let result = step.compensate(&mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn check_wallet_step_name() {
    let wallet = MockWalletClient::new();
    let step = CheckWalletStep {
        wallet_client: Arc::new(wallet),
    };
    assert_eq!(step.name(), "check_wallet");
}

// ──────────────────────────────────────────────────────────────
// CreateOrderStep
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_order_step_execute_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id);
    repo.expect_create()
        .returning(move |_, _, _, _, _| Ok(order.clone()));

    let step = CreateOrderStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_checkout_ctx(titipers_id, jastiper_id);
    let result = step.execute(&mut ctx).await;
    assert!(result.is_ok());
    assert!(ctx.created_order.is_some());
}

#[tokio::test]
async fn create_order_step_execute_gagal_db_error() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    repo.expect_create()
        .returning(|_, _, _, _, _| Err(AppError::Internal));

    let step = CreateOrderStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_checkout_ctx(titipers_id, jastiper_id);
    let result = step.execute(&mut ctx).await;
    assert!(result.is_err());
    assert!(ctx.created_order.is_none());
}

#[tokio::test]
async fn create_order_step_compensate_hapus_order_jika_ada() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    repo.expect_delete().returning(|_| Ok(()));

    let step = CreateOrderStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_checkout_ctx(titipers_id, jastiper_id);
    ctx.created_order = Some(make_order(order_id, titipers_id, jastiper_id));

    let result = step.compensate(&mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn create_order_step_compensate_noop_jika_tidak_ada_order() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let repo = MockOrderRepository::new(); // delete tidak dipanggil
    let step = CreateOrderStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_checkout_ctx(titipers_id, jastiper_id);
    assert!(ctx.created_order.is_none());

    let result = step.compensate(&mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn create_order_step_compensate_gagal_delete_error() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    repo.expect_delete().returning(|_| Err(AppError::Internal));

    let step = CreateOrderStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_checkout_ctx(titipers_id, jastiper_id);
    ctx.created_order = Some(make_order(order_id, titipers_id, jastiper_id));

    let result = step.compensate(&mut ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn create_order_step_name() {
    let repo = MockOrderRepository::new();
    let step = CreateOrderStep {
        order_repo: Arc::new(repo),
    };
    assert_eq!(step.name(), "create_order");
}

// ──────────────────────────────────────────────────────────────
// ReserveStockStep
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn reserve_stock_step_execute_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut inv = MockInventoryClient::new();
    inv.expect_reserve_stock().returning(|_, _, _| Ok(()));

    let step = ReserveStockStep {
        inventory_client: Arc::new(inv),
    };

    let mut ctx = make_checkout_ctx(titipers_id, jastiper_id);
    ctx.created_order = Some(make_order(order_id, titipers_id, jastiper_id));

    let result = step.execute(&mut ctx).await;
    assert!(result.is_ok());
    assert!(ctx.stock_reserved);
}

#[tokio::test]
async fn reserve_stock_step_execute_gagal_tanpa_created_order() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let inv = MockInventoryClient::new();
    let step = ReserveStockStep {
        inventory_client: Arc::new(inv),
    };

    let mut ctx = make_checkout_ctx(titipers_id, jastiper_id);
    assert!(ctx.created_order.is_none());

    let result = step.execute(&mut ctx).await;
    assert!(matches!(result, Err(AppError::Internal)));
}

#[tokio::test]
async fn reserve_stock_step_execute_gagal_inventory_error() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut inv = MockInventoryClient::new();
    inv.expect_reserve_stock()
        .returning(|_, _, _| Err(AppError::Internal));

    let step = ReserveStockStep {
        inventory_client: Arc::new(inv),
    };

    let mut ctx = make_checkout_ctx(titipers_id, jastiper_id);
    ctx.created_order = Some(make_order(order_id, titipers_id, jastiper_id));

    let result = step.execute(&mut ctx).await;
    assert!(result.is_err());
    assert!(!ctx.stock_reserved);
}

#[tokio::test]
async fn reserve_stock_step_compensate_release_jika_reserved() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut inv = MockInventoryClient::new();
    inv.expect_release_stock().returning(|_, _, _| Ok(()));

    let step = ReserveStockStep {
        inventory_client: Arc::new(inv),
    };

    let mut ctx = make_checkout_ctx(titipers_id, jastiper_id);
    ctx.stock_reserved = true;
    ctx.created_order = Some(make_order(order_id, titipers_id, jastiper_id));

    let result = step.compensate(&mut ctx).await;
    assert!(result.is_ok());
    assert!(!ctx.stock_reserved);
}

#[tokio::test]
async fn reserve_stock_step_compensate_noop_jika_tidak_reserved() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let inv = MockInventoryClient::new(); // release tidak dipanggil
    let step = ReserveStockStep {
        inventory_client: Arc::new(inv),
    };

    let mut ctx = make_checkout_ctx(titipers_id, jastiper_id);
    assert!(!ctx.stock_reserved);

    let result = step.compensate(&mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn reserve_stock_step_compensate_noop_jika_tidak_ada_order() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let inv = MockInventoryClient::new();
    let step = ReserveStockStep {
        inventory_client: Arc::new(inv),
    };

    let mut ctx = make_checkout_ctx(titipers_id, jastiper_id);
    ctx.stock_reserved = true;
    ctx.created_order = None; // tidak ada order

    let result = step.compensate(&mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn reserve_stock_step_compensate_gagal_release_error() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut inv = MockInventoryClient::new();
    inv.expect_release_stock()
        .returning(|_, _, _| Err(AppError::Internal));

    let step = ReserveStockStep {
        inventory_client: Arc::new(inv),
    };

    let mut ctx = make_checkout_ctx(titipers_id, jastiper_id);
    ctx.stock_reserved = true;
    ctx.created_order = Some(make_order(order_id, titipers_id, jastiper_id));

    let result = step.compensate(&mut ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn reserve_stock_step_name() {
    let inv = MockInventoryClient::new();
    let step = ReserveStockStep {
        inventory_client: Arc::new(inv),
    };
    assert_eq!(step.name(), "reserve_stock");
}

// ──────────────────────────────────────────────────────────────
// build_checkout_context
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn build_checkout_context_kalkulasi_harga_benar() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let ctx = make_checkout_ctx(titipers_id, jastiper_id);

    // unit_price=50_000, service_fee=5_000, qty=2 → total=(55_000)*2=110_000
    assert_eq!(ctx.unit_price, 50_000);
    assert_eq!(ctx.service_fee, 5_000);
    assert_eq!(ctx.total_price, 110_000);
    assert!(ctx.created_order.is_none());
    assert!(!ctx.stock_reserved);
}
