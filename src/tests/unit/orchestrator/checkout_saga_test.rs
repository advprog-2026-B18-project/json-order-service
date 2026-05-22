use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::order::{CreateOrderRequest, Order};
use crate::models::order_state::OrderStatus;
use crate::models::shipping_address::ShippingAddress;
use crate::orchestrator::SagaStep;
use crate::orchestrator::checkout_saga::{
    CheckWalletStep, CheckoutContext, ReserveStockStep, UpdateStatusToPendingStep,
    build_checkout_context,
};
use crate::repositories::order_repository::MockOrderRepository;
use crate::services::inventory_client::MockInventoryClient;
use crate::services::wallet_client::MockWalletClient;

fn make_order(order_id: Uuid, titipers_id: Uuid, jastiper_id: Uuid, status: OrderStatus) -> Order {
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
        status,
        shipping_address: json!({}),
        note_to_jastiper: None,
        tracking_number: None,
        courier: None,
        cancellation_reason: None,
        cancelled_by: None,
        completed_at: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        expired_at: chrono::Utc::now(),
    }
}

fn make_create_request(product_id: Uuid) -> CreateOrderRequest {
    CreateOrderRequest {
        product_id,
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
    }
}

fn make_checkout_ctx(titipers_id: Uuid, jastiper_id: Uuid) -> CheckoutContext {
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

    build_checkout_context(
        Uuid::new_v4(),
        titipers_id,
        jastiper_id,
        make_create_request(Uuid::new_v4()),
        product,
    )
}

#[tokio::test]
async fn test_check_wallet_step_execute_success_returns_ok() {
    // Arrange
    let mut wallet = MockWalletClient::new();
    wallet.expect_check_wallet().returning(|_, _| Ok(()));
    let step = CheckWalletStep {
        wallet_client: Arc::new(wallet),
    };
    let mut ctx = make_checkout_ctx(Uuid::new_v4(), Uuid::new_v4());

    // Act
    let result = step.execute(&mut ctx).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_check_wallet_step_execute_wallet_error_returns_error() {
    // Arrange
    let mut wallet = MockWalletClient::new();
    wallet.expect_check_wallet().returning(|_, _| {
        Err(AppError::UnprocessableEntity(
            "Saldo tidak cukup".to_string(),
        ))
    });
    let step = CheckWalletStep {
        wallet_client: Arc::new(wallet),
    };
    let mut ctx = make_checkout_ctx(Uuid::new_v4(), Uuid::new_v4());

    // Act
    let result = step.execute(&mut ctx).await;

    // Assert
    assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
}

#[tokio::test]
async fn test_check_wallet_step_compensate_always_returns_ok() {
    // Arrange
    let step = CheckWalletStep {
        wallet_client: Arc::new(MockWalletClient::new()),
    };
    let mut ctx = make_checkout_ctx(Uuid::new_v4(), Uuid::new_v4());

    // Act
    let result = step.compensate(&mut ctx).await;

    // Assert
    assert!(result.is_ok());
    assert_eq!(step.name(), "check_wallet");
}

#[tokio::test]
async fn test_reserve_stock_step_execute_success_sets_stock_reserved() {
    // Arrange
    let mut inventory = MockInventoryClient::new();
    inventory.expect_reserve_stock().returning(|_, _, _| Ok(()));
    let step = ReserveStockStep {
        inventory_client: Arc::new(inventory),
    };
    let mut ctx = make_checkout_ctx(Uuid::new_v4(), Uuid::new_v4());

    // Act
    let result = step.execute(&mut ctx).await;

    // Assert
    assert!(result.is_ok());
    assert!(ctx.stock_reserved);
    assert_eq!(step.name(), "reserve_stock");
}

#[tokio::test]
async fn test_reserve_stock_step_execute_inventory_error_keeps_unreserved() {
    // Arrange
    let mut inventory = MockInventoryClient::new();
    inventory
        .expect_reserve_stock()
        .returning(|_, _, _| Err(AppError::Internal));
    let step = ReserveStockStep {
        inventory_client: Arc::new(inventory),
    };
    let mut ctx = make_checkout_ctx(Uuid::new_v4(), Uuid::new_v4());

    // Act
    let result = step.execute(&mut ctx).await;

    // Assert
    assert!(matches!(result, Err(AppError::Internal)));
    assert!(!ctx.stock_reserved);
}

#[tokio::test]
async fn test_reserve_stock_step_compensate_unreserved_is_noop() {
    // Arrange
    let step = ReserveStockStep {
        inventory_client: Arc::new(MockInventoryClient::new()),
    };
    let mut ctx = make_checkout_ctx(Uuid::new_v4(), Uuid::new_v4());

    // Act
    let result = step.compensate(&mut ctx).await;

    // Assert
    assert!(result.is_ok());
    assert!(!ctx.stock_reserved);
}

#[tokio::test]
async fn test_reserve_stock_step_compensate_reserved_releases_stock() {
    // Arrange
    let mut inventory = MockInventoryClient::new();
    inventory.expect_release_stock().returning(|_, _, _| Ok(()));
    let step = ReserveStockStep {
        inventory_client: Arc::new(inventory),
    };
    let mut ctx = make_checkout_ctx(Uuid::new_v4(), Uuid::new_v4());
    ctx.stock_reserved = true;

    // Act
    let result = step.compensate(&mut ctx).await;

    // Assert
    assert!(result.is_ok());
    assert!(!ctx.stock_reserved);
}

#[tokio::test]
async fn test_reserve_stock_step_compensate_release_error_keeps_reserved() {
    // Arrange
    let mut inventory = MockInventoryClient::new();
    inventory
        .expect_release_stock()
        .returning(|_, _, _| Err(AppError::Internal));
    let step = ReserveStockStep {
        inventory_client: Arc::new(inventory),
    };
    let mut ctx = make_checkout_ctx(Uuid::new_v4(), Uuid::new_v4());
    ctx.stock_reserved = true;

    // Act
    let result = step.compensate(&mut ctx).await;

    // Assert
    assert!(matches!(result, Err(AppError::Internal)));
    assert!(ctx.stock_reserved);
}

#[tokio::test]
async fn test_update_status_to_pending_step_execute_success_stores_order() {
    // Arrange
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let mut repo = MockOrderRepository::new();
    repo.expect_update().returning(move |_, status, _| {
        assert_eq!(*status, OrderStatus::Pending);
        Ok(make_order(
            Uuid::new_v4(),
            titipers_id,
            jastiper_id,
            OrderStatus::Pending,
        ))
    });
    let step = UpdateStatusToPendingStep {
        order_repo: Arc::new(repo),
    };
    let mut ctx = make_checkout_ctx(titipers_id, jastiper_id);

    // Act
    let result = step.execute(&mut ctx).await;

    // Assert
    assert!(result.is_ok());
    assert!(ctx.created_order.is_some());
    assert_eq!(step.name(), "update_status_to_pending");
}

#[tokio::test]
async fn test_update_status_to_pending_step_execute_repo_error_returns_error() {
    // Arrange
    let mut repo = MockOrderRepository::new();
    repo.expect_update()
        .returning(|_, _, _| Err(AppError::Internal));
    let step = UpdateStatusToPendingStep {
        order_repo: Arc::new(repo),
    };
    let mut ctx = make_checkout_ctx(Uuid::new_v4(), Uuid::new_v4());

    // Act
    let result = step.execute(&mut ctx).await;

    // Assert
    assert!(matches!(result, Err(AppError::Internal)));
    assert!(ctx.created_order.is_none());
}

#[tokio::test]
async fn test_update_status_to_pending_step_compensate_success_returns_ok() {
    // Arrange
    let mut repo = MockOrderRepository::new();
    repo.expect_update().returning(|_, status, _| {
        assert_eq!(*status, OrderStatus::Cancelled);
        Ok(make_order(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            OrderStatus::Cancelled,
        ))
    });
    let step = UpdateStatusToPendingStep {
        order_repo: Arc::new(repo),
    };
    let mut ctx = make_checkout_ctx(Uuid::new_v4(), Uuid::new_v4());

    // Act
    let result = step.compensate(&mut ctx).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_update_status_to_pending_step_compensate_repo_error_returns_error() {
    // Arrange
    let mut repo = MockOrderRepository::new();
    repo.expect_update()
        .returning(|_, _, _| Err(AppError::Internal));
    let step = UpdateStatusToPendingStep {
        order_repo: Arc::new(repo),
    };
    let mut ctx = make_checkout_ctx(Uuid::new_v4(), Uuid::new_v4());

    // Act
    let result = step.compensate(&mut ctx).await;

    // Assert
    assert!(matches!(result, Err(AppError::Internal)));
}

#[tokio::test]
async fn test_build_checkout_context_complete_product_calculates_snapshot_and_totals() {
    // Arrange
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let req = make_create_request(product_id);
    let product = json!({
        "name": "Snickers",
        "description": "Coklat",
        "images": ["http://img.url"],
        "originCountry": "Japan",
        "purchaseDate": "2026-01-01",
        "price": 50_000_i64,
        "service_fee": 5_000_i64,
    });

    // Act
    let ctx = build_checkout_context(Uuid::new_v4(), titipers_id, jastiper_id, req, product);

    // Assert
    assert_eq!(ctx.unit_price, 50_000);
    assert_eq!(ctx.service_fee, 5_000);
    assert_eq!(ctx.total_price, 110_000);
    assert_eq!(ctx.snapshot["product_id"], product_id.to_string());
    assert_eq!(ctx.snapshot["image_url"], "http://img.url");
    assert!(!ctx.stock_reserved);
}

#[tokio::test]
async fn test_build_checkout_context_missing_prices_defaults_to_zero() {
    // Arrange
    let req = make_create_request(Uuid::new_v4());
    let product = json!({
        "images": ["http://img.url"]
    });

    // Act
    let ctx = build_checkout_context(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), req, product);

    // Assert
    assert_eq!(ctx.unit_price, 0);
    assert_eq!(ctx.service_fee, 0);
    assert_eq!(ctx.total_price, 0);
}
