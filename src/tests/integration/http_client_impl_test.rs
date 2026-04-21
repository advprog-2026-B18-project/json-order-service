use uuid::Uuid;

use crate::error::AppError;
use crate::ports::auth_client::AuthClient;
use crate::ports::inventory_client::InventoryClient;
use crate::ports::wallet_client::WalletClient;

mockall::mock! {
    pub WalletClientMock {}

    #[async_trait::async_trait]
    impl WalletClient for WalletClientMock {
        async fn deduct_wallet(
            &self,
            user_id: Uuid,
            order_id: Uuid,
            amount: i64,
            description: &str,
        ) -> Result<(), AppError>;

        async fn refund_wallet(
            &self,
            user_id: Uuid,
            order_id: Uuid,
            amount: i64,
            description: &str,
        ) -> Result<(), AppError>;

        async fn check_wallet(&self, user_id: Uuid, req_amount: i64) -> Result<(), AppError>;
    }
}

mockall::mock! {
    pub InventoryClientMock {}

    #[async_trait::async_trait]
    impl InventoryClient for InventoryClientMock {
        async fn reserve_stock(
            &self,
            product_id: Uuid,
            order_id: Uuid,
            quantity: i32,
        ) -> Result<(), AppError>;

        async fn release_stock(
            &self,
            product_id: Uuid,
            order_id: Uuid,
            quantity: i32,
        ) -> Result<(), AppError>;

        async fn fetch_product(&self, product_id: Uuid) -> Result<serde_json::Value, AppError>;

        async fn send_product_rating<'a>(
            &self,
            product_id: Uuid,
            order_id: Uuid,
            rating: f64,
            review: Option<&'a str>,
            product_images: Vec<&'a str>,
        ) -> Result<(), AppError>;
    }
}

mockall::mock! {
    pub AuthClientMock {}

    #[async_trait::async_trait]
    impl AuthClient for AuthClientMock {
        async fn send_jastiper_rating<'a>(
            &self,
            jastiper_id: Uuid,
            order_id: Uuid,
            rating: f64,
            review: Option<&'a str>,
        ) -> Result<(), AppError>;
    }
}

// ─── WalletClient tests ──────────────────────────────────────────────────────

#[tokio::test]
async fn wallet_deduct_sukses_mengembalikan_ok() {
    let mut mock = MockWalletClientMock::new();
    let user_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    mock.expect_deduct_wallet()
        .withf(move |uid, oid, amount, desc| {
            *uid == user_id && *oid == order_id && *amount == 50_000 && *desc == *"Pembayaran order"
        })
        .times(1)
        .returning(|_, _, _, _| Ok(()));

    let result = mock
        .deduct_wallet(user_id, order_id, 50_000, "Pembayaran order")
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn wallet_deduct_saldo_kurang_mengembalikan_error() {
    let mut mock = MockWalletClientMock::new();
    let user_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    mock.expect_deduct_wallet()
        .times(1)
        .returning(|_, _, _, _| Err(AppError::Conflict("Saldo tidak cukup".to_string())));

    let result = mock
        .deduct_wallet(user_id, order_id, 999_999, "Pembayaran order")
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn wallet_refund_sukses() {
    let mut mock = MockWalletClientMock::new();
    let user_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    mock.expect_refund_wallet()
        .withf(move |uid, oid, amount, _| *uid == user_id && *oid == order_id && *amount == 50_000)
        .times(1)
        .returning(|_, _, _, _| Ok(()));

    let result = mock
        .refund_wallet(user_id, order_id, 50_000, "Refund pembatalan")
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn wallet_check_saldo_cukup_ok() {
    let mut mock = MockWalletClientMock::new();
    let user_id = Uuid::new_v4();

    mock.expect_check_wallet()
        .withf(move |uid, amount| *uid == user_id && *amount == 100_000)
        .times(1)
        .returning(|_, _| Ok(()));

    let result = mock.check_wallet(user_id, 100_000).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn wallet_check_saldo_tidak_cukup_error() {
    let mut mock = MockWalletClientMock::new();

    mock.expect_check_wallet()
        .times(1)
        .returning(|_, _| Err(AppError::Conflict("Saldo tidak cukup".to_string())));

    let result = mock.check_wallet(Uuid::new_v4(), 999_999_999).await;
    assert!(result.is_err());
}

// ─── InventoryClient tests ───────────────────────────────────────────────────

#[tokio::test]
async fn inventory_reserve_stock_sukses() {
    let mut mock = MockInventoryClientMock::new();
    let product_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    mock.expect_reserve_stock()
        .withf(move |pid, oid, qty| *pid == product_id && *oid == order_id && *qty == 3)
        .times(1)
        .returning(|_, _, _| Ok(()));

    let result = mock.reserve_stock(product_id, order_id, 3).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn inventory_reserve_stock_stok_habis_error() {
    let mut mock = MockInventoryClientMock::new();

    mock.expect_reserve_stock()
        .times(1)
        .returning(|_, _, _| Err(AppError::Conflict("Stok tidak cukup".to_string())));

    let result = mock
        .reserve_stock(Uuid::new_v4(), Uuid::new_v4(), 100)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn inventory_release_stock_sukses() {
    let mut mock = MockInventoryClientMock::new();
    let product_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    mock.expect_release_stock()
        .withf(move |pid, oid, qty| *pid == product_id && *oid == order_id && *qty == 2)
        .times(1)
        .returning(|_, _, _| Ok(()));

    let result = mock.release_stock(product_id, order_id, 2).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn inventory_fetch_product_mengembalikan_snapshot() {
    use serde_json::json;

    let mut mock = MockInventoryClientMock::new();
    let product_id = Uuid::new_v4();
    let snapshot = json!({ "id": product_id.to_string(), "name": "Sepatu Kulit", "price": 50000 });
    let snapshot_clone = snapshot.clone();

    mock.expect_fetch_product()
        .withf(move |pid| *pid == product_id)
        .times(1)
        .returning(move |_| Ok(snapshot_clone.clone()));

    let result = mock.fetch_product(product_id).await.unwrap();
    assert_eq!(result["name"], "Sepatu Kulit");
    assert_eq!(result["price"], 50000);
}

#[tokio::test]
async fn inventory_fetch_product_tidak_ditemukan_error() {
    let mut mock = MockInventoryClientMock::new();

    mock.expect_fetch_product()
        .times(1)
        .returning(|_| Err(AppError::NotFound("Produk tidak ditemukan".to_string())));

    let result = mock.fetch_product(Uuid::new_v4()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn inventory_send_product_rating_tanpa_review_sukses() {
    let mut mock = MockInventoryClientMock::new();
    let product_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    mock.expect_send_product_rating()
        .withf(move |pid, oid, rating, review, images| {
            *pid == product_id
                && *oid == order_id
                && (*rating - 4.5).abs() < f64::EPSILON
                && review.is_none()
                && images.is_empty()
        })
        .times(1)
        .returning(|_, _, _, _, _| Ok(()));

    let result = mock
        .send_product_rating(product_id, order_id, 4.5, None, vec![])
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn inventory_send_product_rating_dengan_review_dan_gambar_sukses() {
    let mut mock = MockInventoryClientMock::new();

    mock.expect_send_product_rating()
        .times(1)
        .returning(|_, _, _, _, _| Ok(()));

    let result = mock
        .send_product_rating(
            Uuid::new_v4(),
            Uuid::new_v4(),
            5.0,
            Some("Produk sangat bagus, sesuai deskripsi"),
            vec![
                "https://img.example.com/a.jpg",
                "https://img.example.com/b.jpg",
            ],
        )
        .await;

    assert!(result.is_ok());
}

// ─── AuthClient tests ────────────────────────────────────────────────────────

#[tokio::test]
async fn auth_send_jastiper_rating_sukses() {
    let mut mock = MockAuthClientMock::new();
    let jastiper_id = Uuid::new_v4();
    let order_id = Uuid::new_v4();

    mock.expect_send_jastiper_rating()
        .withf(move |jid, oid, rating, review| {
            *jid == jastiper_id
                && *oid == order_id
                && (*rating - 5.0).abs() < f64::EPSILON
                && *review == Some("Jastiper terbaik!")
        })
        .times(1)
        .returning(|_, _, _, _| Ok(()));

    let result = mock
        .send_jastiper_rating(jastiper_id, order_id, 5.0, Some("Jastiper terbaik!"))
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn auth_send_jastiper_rating_tanpa_review_sukses() {
    let mut mock = MockAuthClientMock::new();

    mock.expect_send_jastiper_rating()
        .times(1)
        .returning(|_, _, _, _| Ok(()));

    let result = mock
        .send_jastiper_rating(Uuid::new_v4(), Uuid::new_v4(), 3.0, None)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn auth_send_jastiper_rating_jastiper_tidak_ditemukan_error() {
    let mut mock = MockAuthClientMock::new();

    mock.expect_send_jastiper_rating()
        .times(1)
        .returning(|_, _, _, _| Err(AppError::NotFound("Jastiper tidak ditemukan".to_string())));

    let result = mock
        .send_jastiper_rating(Uuid::new_v4(), Uuid::new_v4(), 4.0, None)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn wallet_deduct_tidak_dipanggil_dua_kali() {
    let mut mock = MockWalletClientMock::new();

    mock.expect_deduct_wallet()
        .times(1)
        .returning(|_, _, _, _| Ok(()));

    let _ = mock
        .deduct_wallet(Uuid::new_v4(), Uuid::new_v4(), 10_000, "Test")
        .await;
}
