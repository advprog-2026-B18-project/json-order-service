use serde_json::json;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::filter_pagination::PaginationParams;
use crate::models::order::{
    CancelRequest, CreateOrderRequest, Order, ShippedRequest, UpdateStatusRequest,
};
use crate::models::order_state::OrderStatus;
use crate::models::role::Role;
use crate::models::shipping_address::ShippingAddress;
use crate::ports::inventory_client::MockInventoryClient;
use crate::ports::order_repository::MockOrderRepository;
use crate::ports::order_status_history_repository::MockOrderStatusHistoryRepository;
use crate::ports::wallet_client::MockWalletClient;
use crate::services::order;

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
        note_to_jastiper: Option::from(String::new()),
        tracking_number: None,
        courier: None,
        cancellation_reason: None,
        cancelled_by: None,
        completed_at: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

fn make_create_request(product_id: Uuid) -> CreateOrderRequest {
    CreateOrderRequest {
        product_id,
        quantity: 1,
        shipping_address: ShippingAddress {
            recipient_name: "Ahmad Fauzan".to_string(),
            phone_number: "081234567890".to_string(),
            street: "Jl. Mawar No. 12, RT 05 RW 03".to_string(),
            kelurahan: "Cipete Selatan".to_string(),
            kecamatan: "Cilandak".to_string(),
            city: "Kota Jakarta Selatan".to_string(),
            province: "DKI Jakarta".to_string(),
            postal_code: "12410".to_string(),
            notes: Some("Kode pos dekat Kantor Lurah, tolong bell apartemen tiga kali".to_string()),
        },
        note_to_jastiper: None,
    }
}

fn make_pagination() -> PaginationParams {
    PaginationParams {
        page: Some(1),
        limit: Some(10),
        sort_by: None,
        order: None,
    }
}

#[tokio::test]
async fn checkout_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();

    let mut inv = MockInventoryClient::new();
    let mut wallet = MockWalletClient::new();
    let mut repo = MockOrderRepository::new();

    let product_json = json!({
        "jastiperId": jastiper_id,
        "name": "Snickers",
        "description": "Coklat",
        "images": ["http://img.url"],
        "origin_country": "Japan",
        "purchase_date": "2026-01-01",
        "price": 10_000_i64,
        "service_fee": 1_000_i64,
    });

    inv.expect_fetch_product()
        .returning(move |_| Ok(product_json.clone()));

    inv.expect_reserve_stock().returning(|_, _, _| Ok(()));

    wallet.expect_check_wallet().returning(|_, _| Ok(()));

    let expected_order = make_order(
        Uuid::new_v4(),
        titipers_id,
        jastiper_id,
        OrderStatus::Pending,
    );
    repo.expect_create()
        .returning(move |_, _, _, _, _| Ok(expected_order.clone()));

    let req = make_create_request(product_id);
    let result = order::checkout(&repo, &inv, &wallet, titipers_id, req).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn checkout_gagal_jastiper_beli_produk_sendiri() {
    let user_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();

    let mut inv = MockInventoryClient::new();
    let wallet = MockWalletClient::new();
    let repo = MockOrderRepository::new();

    let product_json = json!({
        "jastiperId": user_id.to_string(),   // ← Ubah jadi String
        "price": 10_000_i64,
        "service_fee": 1_000_i64,
    });

    inv.expect_fetch_product()
        .returning(move |_| Ok(product_json.clone()));

    let req = make_create_request(product_id);
    let result = order::checkout(&repo, &inv, &wallet, user_id, req).await;

    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn checkout_gagal_fetch_product_error() {
    let titipers_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();

    let mut inv = MockInventoryClient::new();
    let wallet = MockWalletClient::new();
    let repo = MockOrderRepository::new();

    inv.expect_fetch_product()
        .returning(|_| Err(AppError::Internal));

    let req = make_create_request(product_id);
    let result = order::checkout(&repo, &inv, &wallet, titipers_id, req).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn checkout_gagal_check_wallet_release_stock() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();

    let mut inv = MockInventoryClient::new();
    let mut wallet = MockWalletClient::new();
    let repo = MockOrderRepository::new();

    let product_json = json!({
        "jastiperId": jastiper_id,
        "price": 10_000_i64,
        "service_fee": 1_000_i64,
    });

    inv.expect_fetch_product()
        .returning(move |_| Ok(product_json.clone()));

    inv.expect_reserve_stock().returning(|_, _, _| Ok(()));

    inv.expect_release_stock().returning(|_, _, _| Ok(()));

    wallet.expect_check_wallet().returning(|_, _| {
        Err(AppError::UnprocessableEntity(
            "Saldo tidak cukup".to_string(),
        ))
    });

    let req = make_create_request(product_id);
    let result = order::checkout(&repo, &inv, &wallet, titipers_id, req).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn checkout_gagal_create_order_release_stock() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();

    let mut inv = MockInventoryClient::new();
    let mut wallet = MockWalletClient::new();
    let mut repo = MockOrderRepository::new();

    let product_json = json!({
        "jastiperId": jastiper_id,
        "price": 10_000_i64,
        "service_fee": 1_000_i64,
    });

    inv.expect_fetch_product()
        .returning(move |_| Ok(product_json.clone()));

    inv.expect_reserve_stock().returning(|_, _, _| Ok(()));

    wallet.expect_check_wallet().returning(|_, _| Ok(()));

    inv.expect_release_stock().returning(|_, _, _| Ok(()));

    repo.expect_create()
        .returning(|_, _, _, _, _| Err(AppError::Internal));

    let req = make_create_request(product_id);
    let result = order::checkout(&repo, &inv, &wallet, titipers_id, req).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn get_order_sukses_sebagai_titipers() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let expected = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(expected.clone())));

    let result = order::get_order(&repo, order_id, titipers_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn get_order_sukses_sebagai_jastiper() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let expected = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(expected.clone())));

    let result = order::get_order(&repo, order_id, jastiper_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn get_order_gagal_tidak_ditemukan() {
    let mut repo = MockOrderRepository::new();

    repo.expect_find_by_id().returning(|_| Ok(None));

    let result = order::get_order(&repo, Uuid::new_v4(), Uuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn get_order_gagal_bukan_pemilik() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let expected = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(expected.clone())));

    let result = order::get_order(&repo, order_id, orang_lain).await;
    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn update_status_sukses_jastiper_ke_purchased() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    let updated = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(updated.clone()));

    let req = UpdateStatusRequest {
        status: OrderStatus::Purchased,
        notes: None,
        tracking_number: None,
        courier: None,
        cancellation_reason: None,
    };

    let result = order::update_status(&repo, order_id, jastiper_id, &Role::Jastiper, req).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, OrderStatus::Purchased);
}

#[tokio::test]
async fn update_status_gagal_jastiper_bukan_pemilik() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let jastiper_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = UpdateStatusRequest {
        status: OrderStatus::Purchased,
        notes: None,
        tracking_number: None,
        courier: None,
        cancellation_reason: None,
    };

    let result = order::update_status(&repo, order_id, jastiper_lain, &Role::Jastiper, req).await;
    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn update_status_gagal_shipped_tanpa_tracking_number() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = UpdateStatusRequest {
        status: OrderStatus::Shipped,
        notes: None,
        tracking_number: None,
        courier: Some("JNE".to_string()),
        cancellation_reason: None,
    };

    let result = order::update_status(&repo, order_id, jastiper_id, &Role::Jastiper, req).await;
    assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
}

#[tokio::test]
async fn update_status_gagal_shipped_tanpa_courier() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = UpdateStatusRequest {
        status: OrderStatus::Shipped,
        notes: None,
        tracking_number: Some("JNE-123".to_string()),
        courier: None,
        cancellation_reason: None,
    };

    let result = order::update_status(&repo, order_id, jastiper_id, &Role::Jastiper, req).await;
    assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
}

#[tokio::test]
async fn cancel_status_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    let updated = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Refunding);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(updated.clone()));

    let req = UpdateStatusRequest {
        status: OrderStatus::Refunding,
        notes: Some("Dibatalkan".to_string()),
        tracking_number: None,
        courier: None,
        cancellation_reason: Some("Tidak jadi beli".to_string()),
    };

    let result = order::cancel_status(&repo, order_id, jastiper_id, &Role::Jastiper, req).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn cancel_status_gagal_order_tidak_ditemukan() {
    let mut repo = MockOrderRepository::new();

    repo.expect_find_by_id().returning(|_| Ok(None));

    let req = UpdateStatusRequest {
        status: OrderStatus::Refunding,
        notes: None,
        tracking_number: None,
        courier: None,
        cancellation_reason: None,
    };

    let result =
        order::cancel_status(&repo, Uuid::new_v4(), Uuid::new_v4(), &Role::Titipers, req).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn payment_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut wallet = MockWalletClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);
    let paid = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(paid.clone()));

    wallet.expect_deduct_wallet().returning(|_, _, _, _| Ok(()));

    let result = order::payment(&repo, &wallet, titipers_id, order_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn payment_gagal_bukan_pemilik() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let wallet = MockWalletClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let result = order::payment(&repo, &wallet, orang_lain, order_id).await;
    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn payment_gagal_status_bukan_pending() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let wallet = MockWalletClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let result = order::payment(&repo, &wallet, titipers_id, order_id).await;
    assert!(matches!(result, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn payment_gagal_deduct_wallet_error() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut wallet = MockWalletClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    wallet
        .expect_deduct_wallet()
        .returning(|_, _, _, _| Err(AppError::Internal));

    let result = order::payment(&repo, &wallet, titipers_id, order_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn confirm_order_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);
    let completed = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Completed);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(completed.clone()));

    let result = order::confirm_order(&repo, titipers_id, order_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn confirm_order_gagal_bukan_titipers_pemilik() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let titipers_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let result = order::confirm_order(&repo, titipers_lain, order_id).await;
    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn purchased_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    let updated = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(updated.clone()));

    let result = order::purchased(&repo, order_id, jastiper_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn purchased_gagal_bukan_jastiper_pemilik() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let jastiper_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let result = order::purchased(&repo, order_id, jastiper_lain).await;
    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn shipped_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);
    let updated = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Shipped);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));
    repo.expect_update()
        .returning(move |_, _, _| Ok(updated.clone()));

    let req = ShippedRequest {
        tracking_number: Some("JNE-999".to_string()),
        courier: Some("JNE".to_string()),
    };

    let result = order::shipped(&repo, order_id, jastiper_id, req).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn shipped_gagal_tanpa_tracking_number() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Purchased);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let req = ShippedRequest {
        tracking_number: None,
        courier: Some("JNE".to_string()),
    };

    let result = order::shipped(&repo, order_id, jastiper_id, req).await;
    assert!(matches!(result, Err(AppError::UnprocessableEntity(_))));
}

#[tokio::test]
async fn get_order_history_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut history_repo = MockOrderStatusHistoryRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    history_repo
        .expect_get_status_history()
        .returning(|_| Ok(vec![]));

    let result = order::get_order_history(&repo, &history_repo, order_id, titipers_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn get_order_history_gagal_order_tidak_ditemukan() {
    let mut repo = MockOrderRepository::new();
    let history_repo = MockOrderStatusHistoryRepository::new();

    repo.expect_find_by_id().returning(|_| Ok(None));

    let result =
        order::get_order_history(&repo, &history_repo, Uuid::new_v4(), Uuid::new_v4()).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn get_order_history_gagal_bukan_pemilik() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let orang_lain = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let history_repo = MockOrderStatusHistoryRepository::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);

    repo.expect_find_by_id()
        .returning(move |_| Ok(Some(order.clone())));

    let result = order::get_order_history(&repo, &history_repo, order_id, orang_lain).await;
    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn cancel_order_sukses() {
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    let mut repo = MockOrderRepository::new();
    let mut inv = MockInventoryClient::new();
    let mut wallet = MockWalletClient::new();

    let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);
    let mut refunding = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Cancelled);
    refunding.product_snapshot = json!({ "product_id": order.product_id.to_string() });

    // 🔥 Perbaikan: find_by_id dipanggil 2 kali
    let order_clone = order.clone();
    let refunding_clone = refunding.clone();

    let mut call_count = 0;
    repo.expect_find_by_id().returning(move |_| {
        call_count += 1;
        if call_count == 1 {
            Ok(Some(order_clone.clone())) // pertama: Pending
        } else {
            Ok(Some(refunding_clone.clone())) // kedua: Refunding
        }
    });

    // 🔥 Perbaikan: verifikasi parameter update
    repo.expect_update()
        .withf(move |id, status, _params| *id == order_id && status == &OrderStatus::Refunding)
        .returning(move |_, _, _| Ok(refunding.clone()));

    inv.expect_release_stock()
        .withf(move |pid, oid, qty| {
            *pid == order.product_id && *oid == order_id && *qty == order.quantity
        })
        .returning(|_, _, _| Ok(()));

    wallet
        .expect_refund_wallet()
        .withf(move |user_id, oid, amount, _reason| {
            *user_id == titipers_id && *oid == order_id && *amount == order.total_price
        })
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

    // 🔥 Perbaikan: tambah debug
    if let Err(ref e) = result {
        println!("❌ Error: {:?}", e);
    }
    assert!(result.is_ok());
}

#[tokio::test]
async fn cancel_order_gagal_order_tidak_ditemukan() {
    let mut repo = MockOrderRepository::new();
    let inv = MockInventoryClient::new();
    let wallet = MockWalletClient::new();

    repo.expect_find_by_id().returning(|_| Ok(None));

    let req = CancelRequest {
        cancellation_reason: "Test".to_string(),
    };

    let result = order::cancel_order(
        &repo,
        &inv,
        &wallet,
        Uuid::new_v4(),
        Uuid::new_v4(),
        &Role::Titipers,
        req,
    )
    .await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn my_purchases_sukses() {
    let titipers_id = Uuid::new_v4();
    let mut repo = MockOrderRepository::new();

    repo.expect_find_all().returning(|_, _| Ok((vec![], 0)));

    let result = order::my_purchases(&repo, titipers_id, make_pagination()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().1, 0);
}

#[tokio::test]
async fn my_purchases_gagal_db_error() {
    let mut repo = MockOrderRepository::new();

    repo.expect_find_all()
        .returning(|_, _| Err(AppError::Internal));

    let result = order::my_purchases(&repo, Uuid::new_v4(), make_pagination()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn my_sales_sukses() {
    let jastiper_id = Uuid::new_v4();
    let mut repo = MockOrderRepository::new();

    repo.expect_find_all().returning(|_, _| Ok((vec![], 0)));

    let result = order::my_sales(&repo, jastiper_id, make_pagination()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn my_sales_gagal_db_error() {
    let mut repo = MockOrderRepository::new();

    repo.expect_find_all()
        .returning(|_, _| Err(AppError::Internal));

    let result = order::my_sales(&repo, Uuid::new_v4(), make_pagination()).await;
    assert!(result.is_err());
}
