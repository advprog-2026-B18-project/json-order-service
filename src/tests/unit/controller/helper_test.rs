use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

use crate::infrastructure::publisher::MockCheckoutPublisher;
use crate::repositories::idempotency_repository::MockIdempotencyRepository;
use crate::routes::create_app;
use crate::state::AppState;

pub fn make_test_token(user_id: uuid::Uuid, role: &str) -> String {
    use base64::{Engine, engine::general_purpose};
    use jsonwebtoken::{EncodingKey, Header, encode};
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    let raw_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "dGVzdC1zZWNyZXQtdGVzdC1zZWNyZXQ=".to_string());

    let secret_clean: String = raw_secret.chars().filter(|c| !c.is_whitespace()).collect();

    let mut padded = match secret_clean.len() % 4 {
        2 => format!("{}==", secret_clean),
        3 => format!("{}=", secret_clean),
        _ => secret_clean.clone(),
    };

    if padded.ends_with("==") {
        let mut chars: Vec<char> = padded.chars().collect();
        let last_data_idx = padded.len() - 3;
        let last_char = chars[last_data_idx];
        let b64_chars: Vec<char> =
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
                .chars()
                .collect();
        if let Some(idx) = b64_chars.iter().position(|&c| c == last_char) {
            chars[last_data_idx] = b64_chars[idx & !0x3];
        }
        padded = chars.iter().collect();
    }

    let decoded_secret = general_purpose::STANDARD
        .decode(&padded)
        .expect("Failed to decode secret in test");

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    let exp = now + 86400;

    let claims = json!({
        "sub": user_id.to_string(),
        "email": "test@ui.ac.id",
        "role": role,
        "exp": exp,
        "iat": now
    });

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&decoded_secret),
    )
    .unwrap()
}

pub struct TestApp {
    pub router: Router,
}

impl TestApp {
    pub fn new(state: AppState) -> Self {
        let router = create_app(Arc::new(state));
        Self { router }
    }

    pub async fn send(&self, req: Request<Body>) -> (StatusCode, Value) {
        let response = self.router.clone().oneshot(req).await.unwrap();

        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);

        (status, body)
    }
}

pub fn json_request(method: &str, uri: &str, token: &str, body: Option<Value>) -> Request<Body> {
    let body_bytes = body.map(|v| v.to_string().into_bytes()).unwrap_or_default();

    Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(body_bytes))
        .unwrap()
}

pub fn json_request_internal(
    method: &str,
    uri: &str,
    service_key: &str,
) -> axum::http::Request<axum::body::Body> {
    axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .header("X-Service-Key", service_key)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::empty())
        .unwrap()
}

pub fn json_request_internal_post(
    uri: &str,
    service_key: &str,
    body: Option<serde_json::Value>,
) -> axum::http::Request<axum::body::Body> {
    let builder = axum::http::Request::builder()
        .method("POST")
        .uri(uri)
        .header("X-Service-Key", service_key)
        .header("Content-Type", "application/json");

    let body = if let Some(b) = body {
        axum::body::Body::from(b.to_string())
    } else {
        axum::body::Body::empty()
    };

    builder.body(body).unwrap()
}

pub fn dummy_mq_pool() -> deadpool_lapin::Pool {
    let config = deadpool_lapin::Config {
        url: Some("amqp://guest:guest@127.0.0.1:5672/%2f".to_string()),
        ..Default::default()
    };
    config
        .create_pool(Some(deadpool_lapin::Runtime::Tokio1))
        .expect("dummy RabbitMQ pool should be constructible")
}

pub fn noop_checkout_publisher() -> MockCheckoutPublisher {
    let mut publisher = MockCheckoutPublisher::new();
    publisher.expect_publish().returning(|_| Ok(()));
    publisher
}

pub fn noop_idempotency_repo() -> MockIdempotencyRepository {
    MockIdempotencyRepository::new()
}
