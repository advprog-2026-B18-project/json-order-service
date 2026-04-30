mod adapters;
pub mod controller;
mod db;
mod error;
pub mod middleware;
pub mod models;
pub mod orchestrator;
pub mod ports;
pub mod repositories;
mod routes;
pub mod services;
mod state;
#[cfg(test)]
mod tests;

use crate::adapters::auth_client_impl::HttpAuthClient;
use crate::adapters::inventory_client_impl::HttpInventoryClient;
use crate::adapters::wallet_client_impl::HttpWalletClient;
use crate::repositories::order_impl::PgOrderRepository;
use crate::repositories::order_status_history_impl::PgOrderStatusHistoryRepository;
use crate::repositories::rating_jastiper_impl::PgRatingJastiperRepository;
use crate::repositories::rating_product_impl::PgRatingProductRepository;
use crate::routes::create_app;
use crate::state::AppState;
use axum::Router;
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

    let api_router = create_app(state);

    let app = Router::new().merge(api_router);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8084").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
