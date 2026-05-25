use axum::Router;
use axum::body::Body;
use axum::http::Request;
use axum::routing::get;
use std::sync::OnceLock;
use tower::ServiceExt;

use crate::metrics::{MetricsState, metrics_handler};
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_process::Collector;

fn metrics_state() -> &'static MetricsState {
    static STATE: OnceLock<MetricsState> = OnceLock::new();
    STATE.get_or_init(|| {
        let handle = PrometheusBuilder::new()
            .install_recorder()
            .expect("failed to install Prometheus recorder");
        let collector = Collector::default();
        collector.describe();
        MetricsState { handle, collector }
    })
}

#[tokio::test]
async fn test_metrics_endpoint_returns_200() {
    let state = metrics_state().clone();
    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_metrics_endpoint_contains_process_metrics() {
    let state = metrics_state().clone();
    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(
        body_str.contains("process_start_time_seconds"),
        "missing process_start_time_seconds",
    );
    assert!(
        body_str.contains("process_cpu_seconds_total"),
        "missing process_cpu_seconds_total",
    );
    assert!(
        body_str.contains("process_resident_memory_bytes"),
        "missing process_resident_memory_bytes",
    );
    assert!(
        body_str.contains("process_virtual_memory_bytes"),
        "missing process_virtual_memory_bytes",
    );
    if cfg!(target_os = "linux") {
        assert!(
            body_str.contains("process_open_fds"),
            "missing process_open_fds",
        );
    }
}
