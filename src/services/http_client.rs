use crate::error::AppError;
use std::sync::LazyLock;
use std::time::Duration;

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .pool_max_idle_per_host(20)
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(60))
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build shared HTTP client")
});

fn service_key() -> String {
    std::env::var("INTERNAL_SERVICE_KEY").expect("INTERNAL_SERVICE_KEY harus diset di .env")
}

pub async fn internal_post(
    url: &str,
    body: serde_json::Value,
) -> Result<(u16, serde_json::Value), AppError> {
    let response = HTTP_CLIENT
        .post(url)
        .header("X-Service-Key", service_key())
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("HTTP POST failed: {e}");
            AppError::Internal
        })?;

    let status = response.status().as_u16();
    let body: serde_json::Value = response.json().await.map_err(|_| AppError::Internal)?;
    Ok((status, body))
}

pub async fn internal_get(
    url: &str,
    body: serde_json::Value,
) -> Result<(u16, serde_json::Value), AppError> {
    let response = HTTP_CLIENT
        .get(url)
        .header("X-Service-Key", service_key())
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("HTTP GET failed: {e}");
            AppError::Internal
        })?;

    let status = response.status().as_u16();
    let body: serde_json::Value = response.json().await.map_err(|_| AppError::Internal)?;

    Ok((status, body))
}
