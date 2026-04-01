mod db;
mod error;
pub mod controller;
pub mod middleware;
pub mod models;
pub mod repositories;
pub mod services;
#[cfg(test)]
mod tests;

use axum::Router;
use axum::routing::{get, patch, post};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("json_order_service=debug,tower_http=debug")
        .init();

    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL harus diset di .env");
    let pool = db::create_pool(&database_url).await;
    let shared_pool = Arc::new(pool);

    let api_router = Router::new()
        // ORDER ROUTES
        .route("/orders",
               post(controller::order::checkout))
        .route("/orders/{order_id}",
               get(controller::order::get_order))
        .route("/orders/{order_id}/history",
               get(controller::order::get_order_history))
        .route("/orders/{order_id}/status",
               patch(controller::order::update_status))
        .route("/orders/{order_id}/cancel",
               post(controller::order::cancel_order))
        .route("/orders/my/purchases",
               get(controller::order::my_purchases))
        .route("/orders/my/sales",
               get(controller::order::my_sales))

        // RATING ROUTES
        .route("/orders/{order_id}/rating/jastiper",
               get(controller::rating_jastiper::get_rating))
        .route("/orders/{order_id}/rating/jastiper",
               post(controller::rating_jastiper::submit_rating_jastiper))
        .route("/orders/{order_id}/rating/product",
               get(controller::rating_product::get_rating))
        .route("/orders/{order_id}/rating/product",
               post(controller::rating_product::submit_rating_product))

        .with_state(shared_pool);

    let app = Router::new().merge(api_router);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8084")
        .await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
