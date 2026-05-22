use axum::extract::State;
use axum::response::IntoResponse;
use metrics_exporter_prometheus::PrometheusHandle;
use metrics_process::Collector;

#[derive(Clone)]
pub struct MetricsState {
    pub handle: PrometheusHandle,
    pub collector: Collector,
}

pub async fn metrics_handler(State(state): State<MetricsState>) -> impl IntoResponse {
    state.collector.collect();
    state.handle.render()
}
