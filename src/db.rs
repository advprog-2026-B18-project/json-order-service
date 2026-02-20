use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub async fn create_pool(database_url: &str) -> PgPool {
    let pool = PgPoolOptions::new()
        .max_connections(5)
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