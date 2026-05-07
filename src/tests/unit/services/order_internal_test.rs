use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::order::{Order, PaymentConfirmedRequest, RefundConfirmedRequest};
use crate::models::order_state::OrderStatus;
use crate::repositories::order_repository::MockOrderRepository;
use crate::services::order_internal;

fn make_order(order_id: Uuid, titipers_id: Uuid, jastiper_id: Uuid, status: OrderStatus) -> Order {
    Order {
        order_id,
        titipers_id,
        jastiper_id,
        product_id: Uuid::new_v4(),
        product_snapshot: json!({}),
        quantity: 1,
        unit_price: 10_000,
        service_fee: 1_000,
        total_price: 11_000,
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
    }
}

#[tokio::test]
async fn get_order_internal_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let result = order_internal::get_order_internal(Arc::new(repo), order_id).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().order_id, order_id);
}

#[tokio::test]
async fn get_order_internal_gagal_tidak_ditemukan() {
    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

    let result = order_internal::get_order_internal(Arc::new(repo), Uuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn payment_confirmed_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    let paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);

    let mut repo = MockOrderRepository::new();
    let order_clone = order.clone();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order_clone.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(paid.clone()));

    let req = PaymentConfirmedRequest {
        wallet_transaction_id: Uuid::new_v4(),
        amount_deducted: 11_000,
    };

    let result = order_internal::payment_confirmed(Arc::new(repo), order_id, req).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn payment_confirmed_gagal_sudah_paid() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = PaymentConfirmedRequest {
        wallet_transaction_id: Uuid::new_v4(),
        amount_deducted: 11_000,
    };

    let result = order_internal::payment_confirmed(Arc::new(repo), order_id, req).await;
    assert!(matches!(result, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn payment_confirmed_gagal_status_bukan_pending() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);
    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = PaymentConfirmedRequest {
        wallet_transaction_id: Uuid::new_v4(),
        amount_deducted: 11_000,
    };

    let result = order_internal::payment_confirmed(Arc::new(repo), order_id, req).await;
    assert!(matches!(result, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn payment_confirmed_gagal_amount_mismatch() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = PaymentConfirmedRequest {
        wallet_transaction_id: Uuid::new_v4(),
        amount_deducted: 999,
    };

    let result = order_internal::payment_confirmed(Arc::new(repo), order_id, req).await;
    assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
}

#[tokio::test]
async fn payment_confirmed_gagal_order_tidak_ditemukan() {
    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

    let req = PaymentConfirmedRequest {
        wallet_transaction_id: Uuid::new_v4(),
        amount_deducted: 11_000,
    };

    let result = order_internal::payment_confirmed(Arc::new(repo), Uuid::new_v4(), req).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn refund_confirmed_sukses_refunding_ke_cancelled() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);
    let cancelled = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Cancelled);

    let mut repo = MockOrderRepository::new();
    let order_clone = order.clone();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order_clone.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(cancelled.clone()));

    let req = RefundConfirmedRequest {
        success: true,
        wallet_transaction_id: Uuid::new_v4(),
        amount_refunded: 11_000,
        notes: None,
    };

    let result = order_internal::refund_confirmed(Arc::new(repo), order_id, req).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, OrderStatus::Cancelled);
}

#[tokio::test]
async fn refund_confirmed_sukses_refund_failed_ke_refund_failed() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);
    let refund_failed = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::RefundFailed,
    );

    let mut repo = MockOrderRepository::new();
    let order_clone = order.clone();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order_clone.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(refund_failed.clone()));

    let req = RefundConfirmedRequest {
        success: false,
        wallet_transaction_id: Uuid::new_v4(),
        amount_refunded: 0,
        notes: Some("Akun tidak valid".to_string()),
    };

    let result = order_internal::refund_confirmed(Arc::new(repo), order_id, req).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, OrderStatus::RefundFailed);
}

#[tokio::test]
async fn refund_confirmed_gagal_sudah_cancelled() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Cancelled);
    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = RefundConfirmedRequest {
        success: true,
        wallet_transaction_id: Uuid::new_v4(),
        amount_refunded: 11_000,
        notes: None,
    };

    let result = order_internal::refund_confirmed(Arc::new(repo), order_id, req).await;
    assert!(matches!(result, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn refund_confirmed_gagal_status_bukan_refunding() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = RefundConfirmedRequest {
        success: true,
        wallet_transaction_id: Uuid::new_v4(),
        amount_refunded: 11_000,
        notes: None,
    };

    let result = order_internal::refund_confirmed(Arc::new(repo), order_id, req).await;
    assert!(matches!(result, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn refund_confirmed_gagal_amount_mismatch() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);
    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = RefundConfirmedRequest {
        success: true,
        wallet_transaction_id: Uuid::new_v4(),
        amount_refunded: 999,
        notes: None,
    };

    let result = order_internal::refund_confirmed(Arc::new(repo), order_id, req).await;
    assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
}

#[tokio::test]
async fn refund_confirmed_gagal_order_tidak_ditemukan() {
    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id().returning(|_| Ok(None));

    let req = RefundConfirmedRequest {
        success: true,
        wallet_transaction_id: Uuid::new_v4(),
        amount_refunded: 11_000,
        notes: None,
    };

    let result = order_internal::refund_confirmed(Arc::new(repo), Uuid::new_v4(), req).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn refund_confirmed_sukses_tanpa_notes_refund_gagal() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);
    let refund_failed = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::RefundFailed,
    );

    let mut repo = MockOrderRepository::new();
    let order_clone = order.clone();

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order_clone.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(refund_failed.clone()));

    let req = RefundConfirmedRequest {
        success: false,
        wallet_transaction_id: Uuid::new_v4(),
        amount_refunded: 0,
        notes: None,
    };

    let result = order_internal::refund_confirmed(Arc::new(repo), order_id, req).await;
    assert!(result.is_ok());
}
