use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::order::Order;
use crate::models::order_state::OrderStatus;
use crate::orchestrator::SagaStep;
use crate::orchestrator::payment_saga::{DeductWalletStep, PaymentContext, UpdateStatusToPaidStep};
use crate::repositories::order_repository::MockOrderRepository;
use crate::services::wallet_client::{DeductResponse, MockWalletClient};

// ──────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────

fn make_order(order_id: Uuid, titipers_id: Uuid, jastiper_id: Uuid, status: OrderStatus) -> Order {
    Order {
        order_id,
        titipers_id,
        jastiper_id,
        product_id: Uuid::new_v4(),
        product_snapshot: json!({}),
        quantity: 1,
        unit_price: 50_000,
        service_fee: 5_000,
        total_price: 55_000,
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

fn make_payment_ctx(titipers_id: Uuid, order_id: Uuid) -> PaymentContext {
    PaymentContext {
        titipers_id,
        order_id,
        total_price: 55_000,
        wallet_transaction_id: None,
        updated_order: None,
    }
}

// ──────────────────────────────────────────────────────────────
// DeductWalletStep
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn deduct_wallet_execute_sukses() {
    let mut wallet = MockWalletClient::new();
    wallet.expect_deduct_wallet().returning(|_, _, _, _| {
        Ok(DeductResponse {
            transaction_id: "txn-deduct-123".to_string(),
        })
    });

    let step = DeductWalletStep {
        wallet_client: Arc::new(wallet),
    };

    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let mut ctx = make_payment_ctx(titipers_id, order_id);
    let result = step.execute(&mut ctx).await;
    assert!(result.is_ok());
    assert_eq!(
        ctx.wallet_transaction_id,
        Some("txn-deduct-123".to_string())
    );
}

#[tokio::test]
async fn deduct_wallet_execute_gagal_wallet_error() {
    let mut wallet = MockWalletClient::new();
    wallet
        .expect_deduct_wallet()
        .returning(|_, _, _, _| Err(AppError::Internal));

    let step = DeductWalletStep {
        wallet_client: Arc::new(wallet),
    };

    let mut ctx = make_payment_ctx(Uuid::new_v4(), Uuid::new_v4());
    let result = step.execute(&mut ctx).await;
    assert!(result.is_err());
    assert!(ctx.wallet_transaction_id.is_none());
}

#[tokio::test]
async fn deduct_wallet_compensate_noop() {
    let wallet = MockWalletClient::new(); // tidak ada call
    let step = DeductWalletStep {
        wallet_client: Arc::new(wallet),
    };

    let mut ctx = make_payment_ctx(Uuid::new_v4(), Uuid::new_v4());
    let result = step.compensate(&mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn deduct_wallet_name() {
    let wallet = MockWalletClient::new();
    let step = DeductWalletStep {
        wallet_client: Arc::new(wallet),
    };
    assert_eq!(step.name(), "deduct_wallet");
}

// ──────────────────────────────────────────────────────────────
// UpdateStatusToPaidStep
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_status_to_paid_execute_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);

    let mut repo = MockOrderRepository::new();
    let paid_clone = paid.clone();
    repo.expect_update()
        .returning(move |_, _, _| Ok(paid_clone.clone()));

    let step = UpdateStatusToPaidStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_payment_ctx(titipers_id, order_id);
    let result = step.execute(&mut ctx).await;
    assert!(result.is_ok());
    assert!(ctx.updated_order.is_some());
    assert_eq!(ctx.updated_order.unwrap().status, OrderStatus::Paid);
}

#[tokio::test]
async fn update_status_to_paid_execute_gagal_db_error() {
    let mut repo = MockOrderRepository::new();
    repo.expect_update()
        .returning(|_, _, _| Err(AppError::Internal));

    let step = UpdateStatusToPaidStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_payment_ctx(Uuid::new_v4(), Uuid::new_v4());
    let result = step.execute(&mut ctx).await;
    assert!(result.is_err());
    assert!(ctx.updated_order.is_none());
}

#[tokio::test]
async fn update_status_to_paid_compensate_revert_ke_pending() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let pending = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);

    let mut repo = MockOrderRepository::new();
    let pending_clone = pending.clone();
    repo.expect_update()
        .returning(move |_, _, _| Ok(pending_clone.clone()));

    let step = UpdateStatusToPaidStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_payment_ctx(titipers_id, order_id);
    let result = step.compensate(&mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn update_status_to_paid_compensate_gagal_db_error() {
    let mut repo = MockOrderRepository::new();
    repo.expect_update()
        .returning(|_, _, _| Err(AppError::Internal));

    let step = UpdateStatusToPaidStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_payment_ctx(Uuid::new_v4(), Uuid::new_v4());
    let result = step.compensate(&mut ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn update_status_to_paid_name() {
    let repo = MockOrderRepository::new();
    let step = UpdateStatusToPaidStep {
        order_repo: Arc::new(repo),
    };
    assert_eq!(step.name(), "update_status_to_paid");
}
