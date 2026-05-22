use crate::services::adapters::auth_client_adapt::HttpAuthClient;
use crate::services::auth_client::AuthClient;
use serde_json::json;
use uuid::Uuid;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

// === Happy Path ===
#[serial_test::serial]
#[tokio::test]
async fn test_http_auth_client_send_jastiper_rating_success_delegates_to_impl() {
    let mock_server = MockServer::start().await;
    unsafe {
        std::env::set_var("USER_SERVICE_URL", mock_server.uri());
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }

    Mock::given(method("POST"))
        .and(path_regex(r"/internal/users/.+/rating"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = HttpAuthClient;

    let result = client
        .send_jastiper_rating(Uuid::new_v4(), Uuid::new_v4(), 4.5, Some("good"))
        .await;

    assert!(result.is_ok());
}
