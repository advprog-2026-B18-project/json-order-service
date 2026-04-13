#[cfg(test)]
mod tests {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::services::http_client::{internal_get, internal_post};

    // internal_post harus return status code dari server
    #[tokio::test]
    async fn test_internal_post_return_status_200() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/test"))
            .and(header("X-Service-Key", "test-key"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let url = format!("{}/test", server.uri());
        let result = temp_env::async_with_vars(
            [("INTERNAL_SERVICE_KEY", Some("test-key"))],
            async move { internal_post(&url, serde_json::json!({})).await },
        )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 200);
    }

    // internal_post harus return status 422 dari server
    #[tokio::test]
    async fn test_internal_post_return_status_422() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(422))
            .mount(&server)
            .await;

        let url = format!("{}/test", server.uri());
        let result = temp_env::async_with_vars(
            [("INTERNAL_SERVICE_KEY", Some("test-key"))],
            async move { internal_post(&url, serde_json::json!({})).await },
        )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 422);
    }

    // internal_get harus return status dan body JSON
    #[tokio::test]
    async fn test_internal_get_return_body() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "is_sufficient": true })),
            )
            .mount(&server)
            .await;

        let url = format!("{}/test", server.uri());
        let result = temp_env::async_with_vars(
            [("INTERNAL_SERVICE_KEY", Some("test-key"))],
            async move { internal_get(&url, serde_json::json!({})).await },
        )
            .await;

        assert!(result.is_ok());
        let (status, body) = result.unwrap();
        assert_eq!(status, 200);
        assert_eq!(body["is_sufficient"], true);
    }

    // internal_post gagal connect harus return AppError::Internal
    #[tokio::test]
    async fn test_internal_post_gagal_connect() {
        let result = temp_env::async_with_vars(
            [("INTERNAL_SERVICE_KEY", Some("test-key"))],
            async { internal_post("http://localhost:19999/dead", serde_json::json!({})).await },
        )
            .await;

        assert!(result.is_err());
    }
}