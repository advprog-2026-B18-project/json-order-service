use axum::Router;
use json_order_service::db;
use json_order_service::repositories::adapters::order_adapt::PgOrderRepository;
use json_order_service::repositories::adapters::order_status_history_adapt::PgOrderStatusHistoryRepository;
use json_order_service::repositories::adapters::rating_jastiper_adapt::PgRatingJastiperRepository;
use json_order_service::repositories::adapters::rating_product_adapt::PgRatingProductRepository;
use json_order_service::routes::create_app;
use json_order_service::services::adapters::auth_client_adapt::HttpAuthClient;
use json_order_service::services::adapters::inventory_client_adapt::HttpInventoryClient;
use json_order_service::services::adapters::wallet_client_adapt::HttpWalletClient;
use json_order_service::state::AppState;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("json_order_service=debug,tower_http=debug")
        .init();

    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL harus diset di .env");
    let pool = db::create_pool(&database_url).await;

    let order_status_history_repo = Arc::new(PgOrderStatusHistoryRepository::new(pool.clone()));

    let state = Arc::new(AppState {
        order_repo: Arc::new(PgOrderRepository::new(
            pool.clone(),
            order_status_history_repo.clone(),
        )),
        order_status_history_repo,
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
