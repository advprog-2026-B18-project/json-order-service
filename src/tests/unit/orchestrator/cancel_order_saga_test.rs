use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::order::Order;
use crate::models::order_state::OrderStatus;
use crate::models::role::Role;
use crate::orchestrator::SagaStep;
use crate::orchestrator::cancel_order_saga::{
    CancelOrderContext, RefundWalletStep, ReleaseStockStep, UpdateStatusToRefundingStep,
};
use crate::repositories::order_repository::MockOrderRepository;
use crate::services::inventory_client::MockInventoryClient;
use crate::services::wallet_client::{MockWalletClient, RefundResponse};

// ──────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────

fn make_order(order_id: Uuid, titipers_id: Uuid, jastiper_id: Uuid, status: OrderStatus) -> Order {
    Order {
        order_id,
        titipers_id,
        jastiper_id,
        product_id: Uuid::new_v4(),
        product_snapshot: json!({ "product_id": Uuid::new_v4().to_string() }),
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

fn make_cancel_ctx(
    order_id: Uuid,
    titipers_id: Uuid,
    requester_id: Uuid,
    role: Role,
    status: OrderStatus,
) -> CancelOrderContext {
    CancelOrderContext {
        order_id,
        requester_id,
        role,
        product_id: Uuid::new_v4(),
        titipers_id,
        status,
        quantity: 2,
        total_price: 110_000,
        cancellation_reason: "Test cancel".to_string(),
        status_set_to_refunding: false,
        stock_released: false,
        refunding_order: None,
    }
}

// ──────────────────────────────────────────────────────────────
// UpdateStatusToRefundingStep
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_status_to_refunding_execute_sukses_dari_paid() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let paid_order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    let refunding_order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);

    let mut repo = MockOrderRepository::new();
    let paid_clone = paid_order.clone();
    let refunding_clone = refunding_order.clone();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(paid_clone.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(refunding_clone.clone()));

    let step = UpdateStatusToRefundingStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_cancel_ctx(
        order_id,
        titipers_id,
        jastiper_id,
        Role::Jastiper,
        OrderStatus::Paid,
    );
    let result = step.execute(&mut ctx).await;
    assert!(result.is_ok());
    assert!(ctx.status_set_to_refunding);
    assert!(ctx.refunding_order.is_some());
}

#[tokio::test]
async fn update_status_to_refunding_execute_gagal_order_tidak_ditemukan() {
    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

    let step = UpdateStatusToRefundingStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_cancel_ctx(
        order_id,
        titipers_id,
        Uuid::new_v4(),
        Role::Titipers,
        OrderStatus::Pending,
    );
    let result = step.execute(&mut ctx).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn update_status_to_refunding_execute_gagal_status_tidak_valid() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    // Completed tidak bisa di-cancel oleh Jastiper
    let completed_order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);

    let mut repo = MockOrderRepository::new();
    let completed_clone = completed_order.clone();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(completed_clone.clone())));

    let step = UpdateStatusToRefundingStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_cancel_ctx(
        order_id,
        titipers_id,
        jastiper_id,
        Role::Jastiper,
        OrderStatus::Completed,
    );
    let result = step.execute(&mut ctx).await;
    assert!(result.is_err());
    assert!(!ctx.status_set_to_refunding);
}

#[tokio::test]
async fn update_status_to_refunding_compensate_revert_ke_pending() {
    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let pending_order = make_order(order_id, titipers_id, Uuid::new_v4(), OrderStatus::Pending);

    let mut repo = MockOrderRepository::new();
    let pending_clone = pending_order.clone();
    repo.expect_update()
        .returning(move |_, _, _| Ok(pending_clone.clone()));

    let step = UpdateStatusToRefundingStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_cancel_ctx(
        order_id,
        titipers_id,
        Uuid::new_v4(),
        Role::System,
        OrderStatus::Pending,
    );
    ctx.status_set_to_refunding = true;

    let result = step.compensate(&mut ctx).await;
    assert!(result.is_ok());
    assert!(!ctx.status_set_to_refunding);
}

#[tokio::test]
async fn update_status_to_refunding_compensate_noop_jika_belum_diset() {
    let repo = MockOrderRepository::new(); // update tidak dipanggil
    let step = UpdateStatusToRefundingStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_cancel_ctx(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Role::System,
        OrderStatus::Pending,
    );
    assert!(!ctx.status_set_to_refunding);

    let result = step.compensate(&mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn update_status_to_refunding_name() {
    let repo = MockOrderRepository::new();
    let step = UpdateStatusToRefundingStep {
        order_repo: Arc::new(repo),
    };
    assert_eq!(step.name(), "update_status_to_refunding");
}

// ──────────────────────────────────────────────────────────────
// ReleaseStockStep
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn release_stock_execute_sukses() {
    let mut inv = MockInventoryClient::new();
    inv.expect_release_stock().returning(|_, _, _| Ok(()));

    let step = ReleaseStockStep {
        inventory_client: Arc::new(inv),
    };

    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let mut ctx = make_cancel_ctx(
        order_id,
        titipers_id,
        Uuid::new_v4(),
        Role::Jastiper,
        OrderStatus::Paid,
    );

    let result = step.execute(&mut ctx).await;
    assert!(result.is_ok());
    assert!(ctx.stock_released);
}

#[tokio::test]
async fn release_stock_execute_gagal() {
    let mut inv = MockInventoryClient::new();
    inv.expect_release_stock()
        .returning(|_, _, _| Err(AppError::Internal));

    let step = ReleaseStockStep {
        inventory_client: Arc::new(inv),
    };

    let mut ctx = make_cancel_ctx(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Role::Jastiper,
        OrderStatus::Paid,
    );

    let result = step.execute(&mut ctx).await;
    assert!(result.is_err());
    assert!(!ctx.stock_released);
}

#[tokio::test]
async fn release_stock_compensate_reserve_kembali() {
    let mut inv = MockInventoryClient::new();
    inv.expect_reserve_stock().returning(|_, _, _| Ok(()));

    let step = ReleaseStockStep {
        inventory_client: Arc::new(inv),
    };

    let mut ctx = make_cancel_ctx(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Role::System,
        OrderStatus::Paid,
    );
    ctx.stock_released = true;

    let result = step.compensate(&mut ctx).await;
    assert!(result.is_ok());
    assert!(!ctx.stock_released);
}

#[tokio::test]
async fn release_stock_compensate_noop_jika_belum_release() {
    let inv = MockInventoryClient::new(); // reserve tidak dipanggil
    let step = ReleaseStockStep {
        inventory_client: Arc::new(inv),
    };

    let mut ctx = make_cancel_ctx(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Role::System,
        OrderStatus::Paid,
    );
    assert!(!ctx.stock_released);

    let result = step.compensate(&mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn release_stock_compensate_gagal_reserve_error() {
    let mut inv = MockInventoryClient::new();
    inv.expect_reserve_stock()
        .returning(|_, _, _| Err(AppError::Internal));

    let step = ReleaseStockStep {
        inventory_client: Arc::new(inv),
    };

    let mut ctx = make_cancel_ctx(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Role::System,
        OrderStatus::Paid,
    );
    ctx.stock_released = true;

    let result = step.compensate(&mut ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn release_stock_name() {
    let inv = MockInventoryClient::new();
    let step = ReleaseStockStep {
        inventory_client: Arc::new(inv),
    };
    assert_eq!(step.name(), "release_stock");
}

// === Error Path: update fails ===

#[tokio::test]
async fn update_status_to_refunding_execute_update_fails_returns_error() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let paid_order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);

    let mut repo = MockOrderRepository::new();
    let paid_clone = paid_order.clone();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(paid_clone.clone())));
    repo.expect_update()
        .returning(|_, _, _| Err(AppError::Internal));

    let step = UpdateStatusToRefundingStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_cancel_ctx(
        order_id,
        titipers_id,
        jastiper_id,
        Role::Jastiper,
        OrderStatus::Paid,
    );
    let result = step.execute(&mut ctx).await;
    assert!(matches!(result, Err(AppError::Internal)));
    assert!(!ctx.status_set_to_refunding);
}

#[tokio::test]
async fn update_status_to_refunding_compensate_update_fails_returns_error() {
    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    repo.expect_update()
        .returning(|_, _, _| Err(AppError::Internal));

    let step = UpdateStatusToRefundingStep {
        order_repo: Arc::new(repo),
    };

    let mut ctx = make_cancel_ctx(
        order_id,
        titipers_id,
        Uuid::new_v4(),
        Role::System,
        OrderStatus::Pending,
    );
    ctx.status_set_to_refunding = true;

    let result = step.compensate(&mut ctx).await;
    assert!(matches!(result, Err(AppError::Internal)));
}

// ──────────────────────────────────────────────────────────────
// RefundWalletStep
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn refund_wallet_execute_sukses_paid_order() {
    let mut wallet = MockWalletClient::new();
    wallet.expect_refund_wallet().returning(|_, _, _, _| {
        Ok(RefundResponse {
            transaction_id: "".to_string(),
        })
    });

    let step = RefundWalletStep {
        wallet_client: Arc::new(wallet),
    };

    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let mut ctx = make_cancel_ctx(
        order_id,
        titipers_id,
        Uuid::new_v4(),
        Role::Jastiper,
        OrderStatus::Paid, // bukan Pending → refund dipanggil
    );

    let result = step.execute(&mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn refund_wallet_execute_skip_jika_pending() {
    let wallet = MockWalletClient::new(); // refund_wallet tidak dipanggil

    let step = RefundWalletStep {
        wallet_client: Arc::new(wallet),
    };

    let titipers_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    let mut ctx = make_cancel_ctx(
        order_id,
        titipers_id,
        Uuid::new_v4(),
        Role::Jastiper,
        OrderStatus::Pending, // → skip refund
    );

    let result = step.execute(&mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn refund_wallet_execute_gagal_wallet_error() {
    let mut wallet = MockWalletClient::new();
    wallet
        .expect_refund_wallet()
        .returning(|_, _, _, _| Err(AppError::Internal));

    let step = RefundWalletStep {
        wallet_client: Arc::new(wallet),
    };

    let mut ctx = make_cancel_ctx(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Role::Jastiper,
        OrderStatus::Paid,
    );

    let result = step.execute(&mut ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn refund_wallet_compensate_noop() {
    let wallet = MockWalletClient::new();
    let step = RefundWalletStep {
        wallet_client: Arc::new(wallet),
    };

    let mut ctx = make_cancel_ctx(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Role::System,
        OrderStatus::Paid,
    );

    let result = step.compensate(&mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn refund_wallet_name() {
    let wallet = MockWalletClient::new();
    let step = RefundWalletStep {
        wallet_client: Arc::new(wallet),
    };
    assert_eq!(step.name(), "refund_wallet");
}
