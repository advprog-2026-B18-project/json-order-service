#[tokio::test]
#[should_panic(expected = "Gagal koneksi ke Neon DB")]
async fn test_create_pool_invalid_url_panics_before_migration() {
    let database_url = "not-a-postgres-url";
    let _ = crate::db::create_pool(database_url).await;
}
