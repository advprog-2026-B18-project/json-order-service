use serde_json::json;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::order::{CancelRequest, Order, RefundConfirmedRequest};
use crate::models::order_state::OrderStatus;
use crate::models::role::Role;
use crate::ports::inventory_client::MockInventoryClient;
use crate::ports::order_repository::MockOrderRepository;
use crate::ports::wallet_client::MockWalletClient;
use crate::services::order;
use crate::services::order_internal;

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
        total_price: 105_000,
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
async fn integrasi_cancel_paid_lalu_refund_sukses_menjadi_cancelled() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let order_paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    let order_refunding = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);

    let mut repo = MockOrderRepository::new();
    let mut inv = MockInventoryClient::new();
    let mut wallet = MockWalletClient::new();

    let paid_clone = order_paid.clone();
    let refunding_clone = order_refunding.clone();
    let mut call_count = 0;

    repo.expect_find_by_id().returning(move |_| {
        call_count += 1;
        if call_count == 1 {
            Ok(Some(paid_clone.clone()))
        } else {
            Ok(Some(refunding_clone.clone()))
        }
    });

    repo.expect_update()
        .withf(move |id, status, _| *id == order_id && *status == OrderStatus::Refunding)
        .returning(move |_, _, _| Ok(order_refunding.clone()));

    inv.expect_release_stock().returning(|_, _, _| Ok(()));

    wallet.expect_refund_wallet().returning(|_, _, _, _| Ok(()));

    let cancel_req = CancelRequest {
        cancellation_reason: "Produk tidak sesuai".to_string(),
    };

    let cancel_result = order::cancel_order(
        &repo,
        &inv,
        &wallet,
        order_id,
        jastiper_id,
        &Role::Jastiper,
        cancel_req,
    )
    .await;

    assert!(
        cancel_result.is_ok(),
        "Cancel harus sukses: {:?}",
        cancel_result
    );

    let order_refunding_2 = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);
    let order_cancelled = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Cancelled);

    let mut repo2 = MockOrderRepository::new();
    let refunding_2_clone = order_refunding_2.clone();

    repo2
        .expect_find_by_id()
        .returning(move |_| Ok(Some(refunding_2_clone.clone())));

    repo2
        .expect_update()
        .withf(move |id, status, _| *id == order_id && *status == OrderStatus::Cancelled)
        .returning(move |_, _, _| Ok(order_cancelled.clone()));

    let refund_req = RefundConfirmedRequest {
        success: true,
        wallet_transaction_id: Default::default(),
        amount_refunded: order_refunding_2.total_price,
        notes: None,
    };

    let refund_result = order_internal::refund_confirmed(&repo2, order_id, refund_req).await;

    assert!(
        refund_result.is_ok(),
        "Refund confirmed sukses harus ok: {:?}",
        refund_result
    );
    assert_eq!(
        refund_result.unwrap().status,
        OrderStatus::Cancelled,
        "Status akhir harus CANCELLED"
    );
}

#[tokio::test]
async fn integrasi_cancel_paid_lalu_refund_gagal_menjadi_refund_failed() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let order_paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    let order_refunding = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);

    let mut repo = MockOrderRepository::new();
    let mut inv = MockInventoryClient::new();
    let mut wallet = MockWalletClient::new();

    let paid_clone = order_paid.clone();
    let refunding_clone = order_refunding.clone();
    let mut call_count = 0;

    repo.expect_find_by_id().returning(move |_| {
        call_count += 1;
        if call_count == 1 {
            Ok(Some(paid_clone.clone()))
        } else {
            Ok(Some(refunding_clone.clone()))
        }
    });

    repo.expect_update()
        .withf(move |id, status, _| *id == order_id && *status == OrderStatus::Refunding)
        .returning(move |_, _, _| Ok(order_refunding.clone()));

    inv.expect_release_stock().returning(|_, _, _| Ok(()));
    wallet.expect_refund_wallet().returning(|_, _, _, _| Ok(()));

    let cancel_req = CancelRequest {
        cancellation_reason: "Kekeliruan stok".to_string(),
    };

    let cancel_result = order::cancel_order(
        &repo,
        &inv,
        &wallet,
        order_id,
        admin_id,
        &Role::Admin,
        cancel_req,
    )
    .await;

    assert!(
        cancel_result.is_ok(),
        "Cancel oleh Admin harus sukses: {:?}",
        cancel_result
    );

    let order_refunding_2 = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);
    let order_refund_failed = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::RefundFailed,
    );

    let mut repo2 = MockOrderRepository::new();
    let refunding_2_clone = order_refunding_2.clone();

    repo2
        .expect_find_by_id()
        .returning(move |_| Ok(Some(refunding_2_clone.clone())));

    repo2
        .expect_update()
        .withf(move |id, status, _| *id == order_id && *status == OrderStatus::RefundFailed)
        .returning(move |_, _, _| Ok(order_refund_failed.clone()));

    let refund_req = RefundConfirmedRequest {
        success: false,
        wallet_transaction_id: Default::default(),
        amount_refunded: 0,
        notes: Some("Akun tujuan tidak valid".to_string()),
    };

    let refund_result = order_internal::refund_confirmed(&repo2, order_id, refund_req).await;

    assert!(
        refund_result.is_ok(),
        "Refund confirmed gagal tetap harus ok (bukan error): {:?}",
        refund_result
    );
    assert_eq!(
        refund_result.unwrap().status,
        OrderStatus::RefundFailed,
        "Status akhir harus REFUND_FAILED"
    );
}

#[tokio::test]
async fn integrasi_refund_failed_lalu_admin_resolve_ke_cancelled() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let order_failed = make_order(
        order_id,
        titipers_id,
        jastiper_id,
        OrderStatus::RefundFailed,
    );
    let order_cancelled = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Cancelled);

    let mut repo = MockOrderRepository::new();

    let failed_clone = order_failed.clone();

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(failed_clone.clone())));

    repo.expect_update()
        .withf(move |id, status, _| *id == order_id && *status == OrderStatus::Cancelled)
        .returning(move |_, _, _| Ok(order_cancelled.clone()));

    use crate::models::order::UpdateStatusRequest;
    let req = UpdateStatusRequest {
        status: OrderStatus::Cancelled,
        notes: Some("Admin resolve refund gagal secara manual".to_string()),
        tracking_number: None,
        courier: None,
        cancellation_reason: None,
    };

    let result = order::update_status(&repo, order_id, admin_id, &Role::Admin, req).await;

    assert!(
        result.is_ok(),
        "Admin resolve REFUND_FAILED → CANCELLED harus sukses: {:?}",
        result
    );
    assert_eq!(
        result.unwrap().status,
        OrderStatus::Cancelled,
        "Status akhir harus CANCELLED"
    );
}

#[tokio::test]
async fn integrasi_refund_confirmed_ditolak_jika_status_bukan_refunding() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let order_paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);

    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order_paid.clone())));

    let req = RefundConfirmedRequest {
        success: true,
        wallet_transaction_id: Default::default(),
        amount_refunded: 105_000,
        notes: None,
    };

    let result = order_internal::refund_confirmed(&repo, order_id, req).await;

    assert!(
        matches!(result, Err(AppError::Conflict(_))),
        "Harus Conflict karena status bukan REFUNDING/REFUND_FAILED: {:?}",
        result
    );
}

#[tokio::test]
async fn integrasi_refund_confirmed_ditolak_jika_amount_mismatch() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let order_refunding = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);

    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order_refunding.clone())));

    let req = RefundConfirmedRequest {
        success: true,
        wallet_transaction_id: Default::default(),
        amount_refunded: 999,
        notes: None,
    };

    let result = order_internal::refund_confirmed(&repo, order_id, req).await;

    assert!(
        matches!(result, Err(AppError::UnprocessableEntity(_))),
        "Harus UnprocessableEntity karena amount mismatch: {:?}",
        result
    );
}

#[tokio::test]
async fn integrasi_refund_confirmed_ditolak_jika_sudah_cancelled() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let order_cancelled = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Cancelled);

    let mut repo = MockOrderRepository::new();
    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order_cancelled.clone())));

    let req = RefundConfirmedRequest {
        success: true,
        wallet_transaction_id: Default::default(),
        amount_refunded: 105_000,
        notes: None,
    };

    let result = order_internal::refund_confirmed(&repo, order_id, req).await;

    assert!(
        matches!(result, Err(AppError::Conflict(_))),
        "Harus Conflict karena refund sudah pernah dikonfirmasi: {:?}",
        result
    );
}

#[tokio::test]
async fn integrasi_cancel_pending_langsung_cancelled_tanpa_refund() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let order_pending = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    let order_cancelled = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Cancelled);

    let mut repo = MockOrderRepository::new();
    let mut inv = MockInventoryClient::new();
    let mut wallet = MockWalletClient::new();

    let pending_clone = order_pending.clone();

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(pending_clone.clone())));

    repo.expect_update()
        .withf(move |id, status, _| *id == order_id && *status == OrderStatus::Cancelled)
        .returning(move |_, _, _| Ok(order_cancelled.clone()));

    inv.expect_release_stock().returning(|_, _, _| Ok(()));

    wallet.expect_refund_wallet().returning(|_, _, _, _| {
        Err(AppError::NotFound(
            "Order belum dibayar, tidak ada refund".to_string(),
        ))
    });

    let cancel_req = CancelRequest {
        cancellation_reason: "Order tidak jadi".to_string(),
    };

    let result = order::cancel_order(
        &repo,
        &inv,
        &wallet,
        order_id,
        jastiper_id,
        &Role::Jastiper,
        cancel_req,
    )
    .await;

    assert!(
        result.is_ok(),
        "Cancel PENDING oleh Jastiper harus sukses: {:?}",
        result
    );
}
