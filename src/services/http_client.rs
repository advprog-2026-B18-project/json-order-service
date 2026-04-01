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