use crate::models::order::CancelRequest;
use serde_json::json;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::order::{Order, PaymentConfirmedRequest, RefundConfirmedRequest};
use crate::models::order_status_history::OrderStatus;
use crate::models::role::Role;
use crate::ports::inventory_client::MockInventoryClient;
use crate::ports::order_repository::MockOrderRepository;
use crate::ports::wallet_client::MockWalletClient;
use crate::services::{order, order_internal};

fn make_order(
    order_id: Uuid,
    titipers_id: Uuid,
    jastiper_id: Uuid,
    status: OrderStatus,
    total_price: i64,
) -> Order {
    Order {
        order_id,
        titipers_id,
        jastiper_id,
        product_id: Uuid::new_v4(),
        product_snapshot: json!({}),
        quantity: 1,
        unit_price: 10_000,
        service_fee: 1_000,
        total_price,
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
    let order_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let expected = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::Pending,
        11_000,
    );

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(expected.clone())));

    let result = order_internal::get_order_internal(&repo, order_id).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().order_id, order_id);
}

#[tokio::test]
async fn get_order_internal_gagal_tidak_ditemukan() {
    let mut repo = MockOrderRepository::new();

    repo.expect_find_by_id().returning(|_| Ok(None));

    let result = order_internal::get_order_internal(&repo, Uuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn get_order_internal_gagal_db_error() {
    let mut repo = MockOrderRepository::new();

    repo.expect_find_by_id()
        .returning(|_| Err(AppError::Internal));

    let result = order_internal::get_order_internal(&repo, Uuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::Internal)));
}

#[tokio::test]
async fn payment_confirmed_sukses() {
    let order_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::Pending,
        11_000,
    );
    let paid = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::Paid,
        11_000,
    );

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(paid.clone()));

    let req = PaymentConfirmedRequest {
        wallet_transaction_id: Default::default(),
        amount_deducted: 11_000,
    };
    let result = order_internal::payment_confirmed(&repo, order_id, req).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, OrderStatus::Paid);
}

#[tokio::test]
async fn payment_confirmed_gagal_order_tidak_ditemukan() {
    let mut repo = MockOrderRepository::new();

    repo.expect_find_by_id().returning(|_| Ok(None));

    let req = PaymentConfirmedRequest {
        wallet_transaction_id: Default::default(),
        amount_deducted: 11_000,
    };
    let result = order_internal::payment_confirmed(&repo, Uuid::new_v4(), req).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn payment_confirmed_gagal_sudah_paid_conflict() {
    let order_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::Paid,
        11_000,
    );

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = PaymentConfirmedRequest {
        wallet_transaction_id: Default::default(),
        amount_deducted: 11_000,
    };
    let result = order_internal::payment_confirmed(&repo, order_id, req).await;
    assert!(matches!(result, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn payment_confirmed_gagal_status_bukan_pending_conflict() {
    let order_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::Purchased,
        11_000,
    );

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = PaymentConfirmedRequest {
        wallet_transaction_id: Default::default(),
        amount_deducted: 11_000,
    };
    let result = order_internal::payment_confirmed(&repo, order_id, req).await;
    assert!(matches!(result, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn payment_confirmed_gagal_amount_mismatch_422() {
    let order_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::Pending,
        11_000,
    );

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = PaymentConfirmedRequest {
        wallet_transaction_id: Default::default(),
        amount_deducted: 99_999,
    };
    let result = order_internal::payment_confirmed(&repo, order_id, req).await;
    assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
}

#[tokio::test]
async fn payment_confirmed_gagal_db_error_saat_update() {
    let order_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::Pending,
        11_000,
    );

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(|_, _, _| Err(AppError::Internal));

    let req = PaymentConfirmedRequest {
        wallet_transaction_id: Default::default(),
        amount_deducted: 11_000,
    };
    let result = order_internal::payment_confirmed(&repo, order_id, req).await;
    assert!(matches!(result, Err(AppError::Internal)));
}

#[tokio::test]
async fn refund_confirmed_sukses_refund_berhasil() {
    let order_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::Refunding,
        11_000,
    );
    let cancelled = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::Cancelled,
        11_000,
    );

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(cancelled.clone()));

    let req = RefundConfirmedRequest {
        success: true,
        wallet_transaction_id: Default::default(),
        amount_refunded: 11_000,
        notes: None,
    };
    let result = order_internal::refund_confirmed(&repo, order_id, req).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, OrderStatus::Cancelled);
}

#[tokio::test]
async fn refund_confirmed_sukses_refund_gagal_jadi_refund_failed() {
    let order_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::Refunding,
        11_000,
    );
    let refund_failed = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::RefundFailed,
        11_000,
    );

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(refund_failed.clone()));

    let req = RefundConfirmedRequest {
        success: false,
        wallet_transaction_id: Default::default(),
        amount_refunded: 0,
        notes: Some("Rekening tidak valid".to_string()),
    };
    let result = order_internal::refund_confirmed(&repo, order_id, req).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, OrderStatus::RefundFailed);
}

#[tokio::test]
async fn refund_confirmed_gagal_order_tidak_ditemukan() {
    let mut repo = MockOrderRepository::new();

    repo.expect_find_by_id().returning(|_| Ok(None));

    let req = RefundConfirmedRequest {
        success: true,
        wallet_transaction_id: Default::default(),
        amount_refunded: 11_000,
        notes: None,
    };
    let result = order_internal::refund_confirmed(&repo, Uuid::new_v4(), req).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn refund_confirmed_gagal_sudah_cancelled_conflict() {
    let order_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::Cancelled,
        11_000,
    );

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = RefundConfirmedRequest {
        success: true,
        wallet_transaction_id: Default::default(),
        amount_refunded: 11_000,
        notes: None,
    };
    let result = order_internal::refund_confirmed(&repo, order_id, req).await;
    assert!(matches!(result, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn cancel_order_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut inv = MockInventoryClient::new();
    let mut wallet = MockWalletClient::new();

    let order = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::Pending,
        10_000,
    );
    let mut refunding = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::Refunding,
        10_000,
    );
    refunding.product_snapshot = json!({ "product_id": order.product_id.to_string() });

    repo.expect_find_by_id()
        .times(1)
        .returning(move |_| Ok(Some(order.clone())));

    repo.expect_update()
        .times(1)
        .returning(move |_, _, _| Ok(refunding.clone()));

    inv.expect_release_stock()
        .times(1)
        .returning(|_, _, _| Ok(()));

    wallet
        .expect_refund_wallet()
        .times(1)
        .returning(|_, _, _, _| Ok(()));

    let req = CancelRequest {
        cancellation_reason: "Tidak jadi beli".to_string(),
    };

    let result = order::cancel_order(
        &repo,
        &inv,
        &wallet,
        order_id,
        jastiper_id,
        &Role::Jastiper,
        req,
    )
    .await;

    if let Err(ref e) = result {
        println!("Error: {:?}", e);
    }
    assert!(result.is_ok());
}

#[tokio::test]
async fn refund_confirmed_sukses_amount_tidak_dicek_jika_gagal() {
    let order_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();

    let order = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::Refunding,
        11_000,
    );
    let refund_failed = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::RefundFailed,
        11_000,
    );

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(refund_failed.clone()));

    let req = RefundConfirmedRequest {
        success: false,
        wallet_transaction_id: Default::default(),
        amount_refunded: 0,
        notes: None,
    };

    let result = order_internal::refund_confirmed(&repo, order_id, req).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn refund_confirmed_gagal_db_error_saat_update() {
    let order_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::Refunding,
        11_000,
    );

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(|_, _, _| Err(AppError::Internal));

    let req = RefundConfirmedRequest {
        success: true,
        wallet_transaction_id: Default::default(),
        amount_refunded: 11_000,
        notes: None,
    };
    let result = order_internal::refund_confirmed(&repo, order_id, req).await;
    assert!(matches!(result, Err(AppError::Internal)));
}
