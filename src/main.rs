use axum::Router;
use axum::extract::{MatchedPath, Request};
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::routing::get;
use json_order_service::db;
use json_order_service::metrics::{MetricsState, metrics_handler};
use json_order_service::repositories::adapters::order_adapt::PgOrderRepository;
use json_order_service::repositories::adapters::order_status_history_adapt::PgOrderStatusHistoryRepository;
use json_order_service::repositories::adapters::rating_jastiper_adapt::PgRatingJastiperRepository;
use json_order_service::repositories::adapters::rating_product_adapt::PgRatingProductRepository;
use json_order_service::routes::create_app;
use json_order_service::services::adapters::auth_client_adapt::HttpAuthClient;
use json_order_service::services::adapters::inventory_client_adapt::HttpInventoryClient;
use json_order_service::services::adapters::wallet_client_adapt::HttpWalletClient;
use json_order_service::state::AppState;
use metrics::{counter, describe_histogram, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_process::Collector;
use std::sync::Arc;
use std::time::Instant;

// Metrics middleware
// Records http_requests_total and http_request_duration_seconds for every request
async fn track_metrics(req: Request, next: Next) -> impl IntoResponse {
    let start = Instant::now();

    // Prefer the matched route pattern over the raw path.
    // Falls back to raw path only for unmatched routes (404s).
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_owned())
        .unwrap_or_else(|| req.uri().path().to_owned());

    let method = req.method().to_string();

    let response = next.run(req).await;

    let status_code = response.status().as_u16().to_string();
    let latency = start.elapsed().as_secs_f64();

    let labels = [
        ("endpoint", path),
        ("method", method),
        ("status_code", status_code),
    ];

    counter!("http_requests_total", &labels).increment(1);
    histogram!("http_request_duration_seconds", &labels).record(latency);

    response
}

// Prometheus recorder + process metrics setup
// Call once before building the router.
fn setup_metrics() -> MetricsState {
    let buckets = &[
        0.001, 0.002, 0.004, 0.008, 0.016, 0.032, 0.064, 0.128, 0.256, 0.512, 1.024, 2.048, 4.096,
        8.192, 16.384,
    ];

    let handle = PrometheusBuilder::new()
        .set_buckets(buckets)
        .expect("invalid histogram buckets")
        .install_recorder()
        .expect("failed to install Prometheus recorder");

    describe_histogram!(
        "http_request_duration_seconds",
        "HTTP request duration in seconds"
    );

    let collector = Collector::default();
    collector.describe();

    // Process metrics are collected at scrape time in metrics_handler,
    // so no background polling task is needed.

    MetricsState { handle, collector }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("json_order_service=debug,tower_http=debug")
        .init();

    dotenvy::dotenv().ok();

    let metrics_state = setup_metrics();

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

    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(metrics_state)
        .merge(api_router)
        .route_layer(middleware::from_fn(track_metrics));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8084").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
