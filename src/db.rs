use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

pub async fn create_pool(database_url: &str) -> PgPool {
    let pool = PgPoolOptions::new()
        .max_connections(30)
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .idle_timeout(Duration::from_secs(30))
        .max_lifetime(Duration::from_secs(1800))
        .connect(database_url)
        .await
        .expect("Gagal koneksi ke Neon DB");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Gagal menjalankan migration");

    println!("Migration selesai!");

    pool
}
