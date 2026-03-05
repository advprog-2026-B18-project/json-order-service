mod base;
mod db;
mod handlers;
mod models;
mod repositories;
#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL harus diset di .env");

    let pool = db::create_pool(&database_url).await;
    println!("Berhasil konek ke Neon DB!");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    // Test koneksi dengan ping — tidak butuh tabel apapun
    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .expect("Gagal ping database");

    println!("Koneksi ke DB modul 3 berjalan normal!");
}
