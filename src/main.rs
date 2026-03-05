mod base;
mod db;
mod error;
mod handlers;
mod models;
mod repositories;
#[cfg(test)]
mod tests;

use std::sync::Arc;
use axum::Router;
use axum::routing::get;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::order::my_purchases,
        crate::handlers::order::my_sales,
    ),
    tags(
        (name = "Orders", description = "Order management endpoints")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL harus diset di .env");

    let pool = db::create_pool(&database_url).await;
    println!("✅ Berhasil konek ke Neon DB!");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .expect("Gagal ping database");
    println!("✅ Koneksi ke DB berjalan normal!");

    let shared_pool = Arc::new(pool);

    // Router API dengan state (pool)
    let api_router = Router::new()
        .route("/orders/my/purchases", get(handlers::order::my_purchases))
        .route("/orders/my/sales", get(handlers::order::my_sales))
        .with_state(shared_pool);

    let app: Router = Router::new()
        .merge(api_router)
        .merge(Scalar::with_url("/scalar", ApiDoc::openapi()));

    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Gagal bind ke port 3000");

    println!("🚀 Server berjalan di http://localhost:3000");
    println!("📖 Swagger UI (Scalar)  →  http://localhost:3000/scalar");
    println!("📄 OpenAPI JSON →  http://localhost:3000/api-docs/openapi.json");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}