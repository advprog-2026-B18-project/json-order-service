use axum::Router;
use deadpool_lapin::{Config, Runtime};
use json_order_service::db;
use json_order_service::infrastructure::publisher::RabbitMqCheckoutPublisher;
use json_order_service::infrastructure::worker::run_worker;
use json_order_service::repositories::adapters::idempotency_adapt::PgIdempotencyRepository;
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
use uuid::Uuid;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("json_order_service=debug,tower_http=debug")
        .init();

    dotenvy::dotenv().ok();

    // ── DB pool ───────────────────────────────────────────────────
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL harus diset di .env");
    let pool = db::create_pool(&database_url).await;

    // ── RabbitMQ pool ─────────────────────────────────────────────
    let amqp_url = std::env::var("AMQP_URL").expect("AMQP_URL harus diset di .env");
    let mq_config = Config {
        url: Some(amqp_url),
        ..Default::default()
    };
    let mq_pool = mq_config
        .create_pool(Some(Runtime::Tokio1))
        .expect("gagal buat RabbitMQ pool");

    // ── Repositories ──────────────────────────────────────────────
    let order_status_history_repo = Arc::new(PgOrderStatusHistoryRepository::new(pool.clone()));
    let idempotency_repo = Arc::new(PgIdempotencyRepository::new(pool.clone()));

    // ── AppState ──────────────────────────────────────────────────
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
        checkout_publisher: Arc::new(RabbitMqCheckoutPublisher::new(mq_pool.clone())),
        mq_pool: mq_pool.clone(),
        idempotency_repo,
    });

    // ── Spawn worker ──────────────────────────────────────────────
    {
        let mq = mq_pool.clone();
        let order_repo = Arc::clone(&state.order_repo);
        let inventory = Arc::clone(&state.inventory_client);
        let wallet = Arc::clone(&state.wallet_client);
        let idempotency = Arc::clone(&state.idempotency_repo);
        tokio::spawn(async move {
            if let Err(e) = run_worker(mq, order_repo, inventory, wallet, idempotency).await {
                tracing::error!("worker crashed: {e}");
            }
        });
    }

    // ── Spawn DB sweep auto-cancel ────────────────────────────────
    {
        let order_repo = Arc::clone(&state.order_repo);
        let inventory = Arc::clone(&state.inventory_client);
        let wallet = Arc::clone(&state.wallet_client);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                match order_repo.find_expired_pending_orders().await {
                    Ok(orders) => {
                        for o in orders {
                            let repo = Arc::clone(&order_repo);
                            let inv = Arc::clone(&inventory);
                            let wal = Arc::clone(&wallet);
                            tokio::spawn(async move {
                                let _ = json_order_service::services::order::cancel_order(
                                    repo,
                                    inv,
                                    wal,
                                    o.order_id,
                                    Uuid::default(),
                                    &json_order_service::models::role::Role::System,
                                    json_order_service::models::order::CancelRequest {
                                        cancellation_reason: "Timeout 15 menit, auto-cancel"
                                            .to_string(),
                                    },
                                )
                                .await;
                            });
                        }
                    }
                    Err(e) => tracing::error!("sweep error: {e}"),
                }
            }
        });
    }

    // ── Axum server ───────────────────────────────────────────────
    let api_router = create_app(state);
    let app = Router::new().merge(api_router);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8084").await.unwrap();
    tracing::info!("order service listening on :8084");
    axum::serve(listener, app).await.unwrap();
}
