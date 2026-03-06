mod base;
mod db;
mod error;
mod handlers;
mod middleware;
mod models;
mod repositories;
#[cfg(test)]
mod tests;

use axum::Router;
use axum::routing::{get, patch, post};
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::order::my_purchases,
        crate::handlers::order::my_sales,
        crate::handlers::order::get_order,
        crate::handlers::order::get_order_history,
        crate::handlers::order::checkout,
        crate::handlers::order::update_status,
        crate::handlers::order::cancel_order,
    ),
    tags(
        (name = "Orders", description = "Order management endpoints")
    )
)]
struct ApiDoc;

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

    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .expect("Gagal ping database");
    println!("Koneksi ke DB berjalan normal!");

    let shared_pool = Arc::new(pool);

    let api_router = Router::new()
        .route("/orders", post(handlers::order::checkout))
        .route("/orders/my/purchases", get(handlers::order::my_purchases))
        .route("/orders/my/sales", get(handlers::order::my_sales))
        .route("/orders/{order_id}", get(handlers::order::get_order))
        .route(
            "/orders/{order_id}/history",
            get(handlers::order::get_order_history),
        )
        .route(
            "/orders/{order_id}/status",
            patch(handlers::order::update_status),
        )
        .route(
            "/orders/{order_id}/cancel",
            post(handlers::order::cancel_order),
        )
        .with_state(shared_pool);

    let app: Router = Router::new()
        .merge(api_router)
        .merge(Scalar::with_url("/scalar", ApiDoc::openapi()));

    let addr = "0.0.0.0:8084";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Gagal bind ke port 3000");

    println!("Swagger UI (Scalar)  →  http://localhost:8084/scalar");

    axum::serve(listener, app).await.expect("Server error");
}
