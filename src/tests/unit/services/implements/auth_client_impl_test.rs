#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use crate::services::implements::auth_client_impl::send_jastiper_rating;
    use uuid::Uuid;
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn setup_env(base_url: &str) {
        unsafe {
            std::env::set_var("USER_SERVICE_URL", base_url);
        }
        unsafe {
            std::env::set_var("INTERNAL_SERVICE_KEY", "test-internal-key");
        }
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn send_jastiper_rating_sukses_200() {
        let mock_server = MockServer::start().await;
        setup_env(&mock_server.uri());

        Mock::given(method("POST"))
            .and(path_regex(r"/internal/users/.+/rating"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&mock_server)
            .await;

        let result = send_jastiper_rating(
            Uuid::new_v4(),
            Uuid::new_v4(),
            4.5,
            Some("Jastiper sangat ramah"),
        )
        .await;

        assert!(result.is_ok());
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn send_jastiper_rating_sukses_dengan_review_none() {
        let mock_server = MockServer::start().await;
        setup_env(&mock_server.uri());

        Mock::given(method("POST"))
            .and(path_regex(r"/internal/users/.+/rating"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&mock_server)
            .await;

        let result = send_jastiper_rating(Uuid::new_v4(), Uuid::new_v4(), 5.0, None).await;

        assert!(result.is_ok());
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn send_jastiper_rating_404_non_fatal() {
        let mock_server = MockServer::start().await;
        setup_env(&mock_server.uri());

        Mock::given(method("POST"))
            .and(path_regex(r"/internal/users/.+/rating"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({})))
            .mount(&mock_server)
            .await;

        let result = send_jastiper_rating(Uuid::new_v4(), Uuid::new_v4(), 4.0, None).await;

        assert!(result.is_ok());
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn send_jastiper_rating_409_idempotent() {
        let mock_server = MockServer::start().await;
        setup_env(&mock_server.uri());

        Mock::given(method("POST"))
            .and(path_regex(r"/internal/users/.+/rating"))
            .respond_with(ResponseTemplate::new(409).set_body_json(serde_json::json!({})))
            .mount(&mock_server)
            .await;

        let result =
            send_jastiper_rating(Uuid::new_v4(), Uuid::new_v4(), 3.5, Some("Oke lah")).await;

        assert!(result.is_ok());
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn send_jastiper_rating_500_unexpected_tetap_ok() {
        let mock_server = MockServer::start().await;
        setup_env(&mock_server.uri());

        Mock::given(method("POST"))
            .and(path_regex(r"/internal/users/.+/rating"))
            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({})))
            .mount(&mock_server)
            .await;

        let result = send_jastiper_rating(Uuid::new_v4(), Uuid::new_v4(), 4.0, None).await;

        assert!(result.is_ok());
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn send_jastiper_rating_network_error() {
        setup_env("http://127.0.0.1:19999");

        let result = send_jastiper_rating(Uuid::new_v4(), Uuid::new_v4(), 4.0, None).await;

        assert!(matches!(result, Err(AppError::Internal)));
    }
}
