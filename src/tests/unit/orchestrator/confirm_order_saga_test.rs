use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::order::Order;
use crate::models::order_state::OrderStatus;
use crate::orchestrator::SagaStep;
use crate::orchestrator::confirm_order_saga::{
    ConfirmOrderContext, SendConfirmationProductStep, TransferEarningsStep,
    UpdateStatusToCompletedStep,
};
use crate::repositories::order_repository::MockOrderRepository;
use crate::services::inventory_client::MockInventoryClient;
use crate::services::wallet_client::{EarningsResponse, MockWalletClient};

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

fn make_confirm_ctx(
    titipers_id: Uuid,
    jastiper_id: Uuid,
    order_id: Uuid,
    product_id: Uuid,
) -> ConfirmOrderContext {
    ConfirmOrderContext {
        titipers_id,
        jastiper_id,
        order_id,
        product_id,
        total_price: 55_000,
        earnings_transaction_id: None,
        updated_order: None,
    }
}

// ──────────────────────────────────────────────────────────────
// UpdateStatusToCompletedStep
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_status_to_completed_execute_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let completed = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);

    let mut repo = MockOrderRepository::new();
    let completed_clone = completed.clone();
    repo.expect_update()
        .returning(move |_, _, _| Ok(completed_clone.clone()));

    let step = UpdateStatusToCompletedStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_confirm_ctx(titipers_id, jastiper_id, order_id, Uuid::new_v4());
    let result = step.execute(&mut ctx).await;
    assert!(result.is_ok());
    assert!(ctx.updated_order.is_some());
    assert_eq!(ctx.updated_order.unwrap().status, OrderStatus::Completed);
}

#[tokio::test]
async fn update_status_to_completed_execute_gagal_db_error() {
    let mut repo = MockOrderRepository::new();
    repo.expect_update()
        .returning(|_, _, _| Err(AppError::Internal));

    let step = UpdateStatusToCompletedStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_confirm_ctx(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let result = step.execute(&mut ctx).await;
    assert!(result.is_err());
    assert!(ctx.updated_order.is_none());
}

#[tokio::test]
async fn update_status_to_completed_compensate_revert_ke_shipped() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let shipped = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);

    let mut repo = MockOrderRepository::new();
    let shipped_clone = shipped.clone();
    repo.expect_update()
        .returning(move |_, _, _| Ok(shipped_clone.clone()));

    let step = UpdateStatusToCompletedStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_confirm_ctx(titipers_id, jastiper_id, order_id, Uuid::new_v4());
    let result = step.compensate(&mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn update_status_to_completed_compensate_gagal_db_error() {
    let mut repo = MockOrderRepository::new();
    repo.expect_update()
        .returning(|_, _, _| Err(AppError::Internal));

    let step = UpdateStatusToCompletedStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_confirm_ctx(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let result = step.compensate(&mut ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn update_status_to_completed_name() {
    let repo = MockOrderRepository::new();
    let step = UpdateStatusToCompletedStep {
        order_repo: Arc::new(repo),
    };
    assert_eq!(step.name(), "update_status_to_completed");
}

// ──────────────────────────────────────────────────────────────
// TransferEarningsStep
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn transfer_earnings_execute_sukses() {
    let mut wallet = MockWalletClient::new();
    wallet.expect_earnings_wallet().returning(|_, _, _| {
        Ok(EarningsResponse {
            transaction_id: "txn-abc".to_string(),
        })
    });

    let step = TransferEarningsStep {
        wallet_client: Arc::new(wallet),
    };

    let mut ctx = make_confirm_ctx(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let result = step.execute(&mut ctx).await;
    assert!(result.is_ok());
    assert_eq!(ctx.earnings_transaction_id, Some("txn-abc".to_string()));
}

#[tokio::test]
async fn transfer_earnings_execute_gagal_wallet_error() {
    let mut wallet = MockWalletClient::new();
    wallet
        .expect_earnings_wallet()
        .returning(|_, _, _| Err(AppError::Internal));

    let step = TransferEarningsStep {
        wallet_client: Arc::new(wallet),
    };

    let mut ctx = make_confirm_ctx(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let result = step.execute(&mut ctx).await;
    assert!(result.is_err());
    assert!(ctx.earnings_transaction_id.is_none());
}

#[tokio::test]
async fn transfer_earnings_compensate_reverse_jika_ada_txn() {
    let mut wallet = MockWalletClient::new();
    wallet
        .expect_reverse_earnings()
        .returning(|_, _, _, _| Ok(()));

    let step = TransferEarningsStep {
        wallet_client: Arc::new(wallet),
    };

    let mut ctx = make_confirm_ctx(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    ctx.earnings_transaction_id = Some("txn-abc".to_string());

    let result = step.compensate(&mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn transfer_earnings_compensate_noop_jika_tidak_ada_txn() {
    let wallet = MockWalletClient::new(); // reverse_earnings tidak dipanggil
    let step = TransferEarningsStep {
        wallet_client: Arc::new(wallet),
    };

    let mut ctx = make_confirm_ctx(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    assert!(ctx.earnings_transaction_id.is_none());

    let result = step.compensate(&mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn transfer_earnings_compensate_gagal_reverse_error() {
    let mut wallet = MockWalletClient::new();
    wallet
        .expect_reverse_earnings()
        .returning(|_, _, _, _| Err(AppError::Internal));

    let step = TransferEarningsStep {
        wallet_client: Arc::new(wallet),
    };

    let mut ctx = make_confirm_ctx(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    ctx.earnings_transaction_id = Some("txn-xyz".to_string());

    let result = step.compensate(&mut ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn transfer_earnings_name() {
    let wallet = MockWalletClient::new();
    let step = TransferEarningsStep {
        wallet_client: Arc::new(wallet),
    };
    assert_eq!(step.name(), "transfer_earnings_to_jastiper");
}

// ──────────────────────────────────────────────────────────────
// SendConfirmationProductStep
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn send_confirmation_execute_sukses() {
    let mut inv = MockInventoryClient::new();
    inv.expect_confirm_order_received().returning(|_, _| Ok(()));

    let step = SendConfirmationProductStep {
        inventory_client: Arc::new(inv),
    };

    let mut ctx = make_confirm_ctx(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let result = step.execute(&mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn send_confirmation_execute_gagal_inventory_error() {
    let mut inv = MockInventoryClient::new();
    inv.expect_confirm_order_received()
        .returning(|_, _| Err(AppError::Internal));

    let step = SendConfirmationProductStep {
        inventory_client: Arc::new(inv),
    };

    let mut ctx = make_confirm_ctx(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let result = step.execute(&mut ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn send_confirmation_compensate_noop() {
    let inv = MockInventoryClient::new();
    let step = SendConfirmationProductStep {
        inventory_client: Arc::new(inv),
    };

    let mut ctx = make_confirm_ctx(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let result = step.compensate(&mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn send_confirmation_name() {
    let inv = MockInventoryClient::new();
    let step = SendConfirmationProductStep {
        inventory_client: Arc::new(inv),
    };
    assert_eq!(step.name(), "send_confirmation_product");
}
