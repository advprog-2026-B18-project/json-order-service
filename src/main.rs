pub mod controller;
mod db;
mod error;
pub mod middleware;
pub mod models;
pub mod ports;
pub mod repositories;
pub mod services;
mod state;
#[cfg(test)]
mod tests;
mod adapters;

use crate::repositories::order_impl::PgOrderRepository;
use crate::repositories::order_status_history_impl::PgOrderStatusHistoryRepository;
use crate::repositories::rating_jastiper_impl::PgRatingJastiperRepository;
use crate::repositories::rating_product_impl::PgRatingProductRepository;
use crate::adapters::auth_client_impl::HttpAuthClient;
use crate::adapters::inventory_client_impl::HttpInventoryClient;
use crate::adapters::wallet_client_impl::HttpWalletClient;
use crate::state::AppState;
use axum::Router;
use axum::routing::{get, patch, post};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("json_order_service=debug,tower_http=debug")
        .init();

    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL harus diset di .env");
    let pool = db::create_pool(&database_url).await;

    let state = Arc::new(AppState {
        order_repo: Arc::new(PgOrderRepository::new(pool.clone())),
        order_status_history_repo: Arc::new(PgOrderStatusHistoryRepository::new(pool.clone())),
        rating_product_repo: Arc::new(PgRatingProductRepository::new(pool.clone())),
        rating_jastiper_repo: Arc::new(PgRatingJastiperRepository::new(pool.clone())),

        inventory_client: Arc::new(HttpInventoryClient),
        wallet_client: Arc::new(HttpWalletClient),
        auth_client: Arc::new(HttpAuthClient),
    });

    let api_router = Router::new()
        // ORDER
        .route("/orders", post(controller::order::checkout))
        .route("/orders/:order_id", get(controller::order::get_order))
        .route(
            "/orders/:order_id/payment",
            patch(controller::order::payment),
        )
        .route(
            "/orders/:order_id/confirm",
            patch(controller::order::confirm_order),
        )
        .route(
            "/orders/:order_id/purchased",
            patch(controller::order::purchased),
        )
        .route(
            "/orders/:order_id/shipped",
            patch(controller::order::shipped),
        )
        .route(
            "/orders/:order_id/history",
            get(controller::order::get_order_history),
        )
        .route(
            "/orders/:order_id/cancel",
            post(controller::order::cancel_order),
        )
        .route("/orders/my/purchases", get(controller::order::my_purchases))
        .route("/orders/my/sales", get(controller::order::my_sales))
        // RATING
        .route(
            "/orders/:order_id/rating/jastiper",
            get(controller::rating_jastiper::get_rating),
        )
        .route(
            "/orders/:order_id/rating/jastiper",
            post(controller::rating_jastiper::submit_rating_jastiper),
        )
        .route(
            "/orders/:order_id/rating/product",
            get(controller::rating_product::get_rating),
        )
        .route(
            "/orders/:order_id/rating/product",
            post(controller::rating_product::submit_rating_product),
        )
        // INTERNAL
        .route(
            "/internal/orders/:order_id/payment-info",
            get(controller::internal::payment_info),
        )
        .route(
            "/internal/orders/:order_id/payment-confirmed",
            post(controller::internal::payment_confirmed),
        )
        .route(
            "/internal/orders/:order_id/refund-confirmed",
            post(controller::internal::refund_confirmed),
        )
        .with_state(state);

    let app = Router::new().merge(api_router);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8084").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
