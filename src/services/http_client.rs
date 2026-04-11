use serde_json::json;
use uuid::Uuid;
use crate::error::AppError;

fn service_key() -> String {
    std::env::var("INTERNAL_SERVICE_KEY").expect("INTERNAL_SERVICE_KEY harus diset di .env")
}

pub(crate) async fn internal_post(url: &str, body: serde_json::Value) -> Result<u16, AppError> {
    let status = reqwest::Client::new()
        .post(url)
        .header("X-Service-Key", service_key())
        .json(&body)
        .send()
        .await
        .map_err(|_| AppError::Internal)?
        .status()
        .as_u16();
    Ok(status)
}

pub(crate) async fn internal_get(url: &str, body: serde_json::Value) -> Result<(u16, serde_json::Value), AppError> {
    let response = reqwest::Client::new()
        .get(url)
        .header("X-Service-Key", service_key())
        .json(&body)
        .send()
        .await
        .map_err(|_| AppError::Internal)?;

    let status = response.status().as_u16();
    let body: serde_json::Value = response.json().await.map_err(|_| AppError::Internal)?;

    Ok((status, body))
}
