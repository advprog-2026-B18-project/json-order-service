#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::services::auth_client::send_jastiper_rating;

    // send_jastiper_rating berhasil (200)
    #[tokio::test]
    async fn test_send_jastiper_rating_berhasil() {
        let server = MockServer::start().await;
        let jastiper_id = Uuid::new_v4();

        Mock::given(method("POST"))
            .and(path(format!("/internal/users/{}/rating", jastiper_id)))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let order_id = Uuid::new_v4();
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("USER_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { send_jastiper_rating(jastiper_id, order_id, 4.5, Some("Bagus")).await },
        )
        .await;

        assert!(result.is_ok());
    }

    // send_jastiper_rating jastiper tidak ditemukan (404) → Ok (non-fatal)
    #[tokio::test]
    async fn test_send_jastiper_rating_404_nonfatal() {
        let server = MockServer::start().await;
        let jastiper_id = Uuid::new_v4();

        Mock::given(method("POST"))
            .and(path(format!("/internal/users/{}/rating", jastiper_id)))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let order_id = Uuid::new_v4();
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("USER_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { send_jastiper_rating(jastiper_id, order_id, 4.5, None).await },
        )
        .await;

        assert!(result.is_ok()); // non-fatal
    }

    // send_jastiper_rating idempotent (409) → Ok
    #[tokio::test]
    async fn test_send_jastiper_rating_idempotent() {
        let server = MockServer::start().await;
        let jastiper_id = Uuid::new_v4();

        Mock::given(method("POST"))
            .and(path(format!("/internal/users/{}/rating", jastiper_id)))
            .respond_with(ResponseTemplate::new(409))
            .mount(&server)
            .await;

        let order_id = Uuid::new_v4();
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("USER_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { send_jastiper_rating(jastiper_id, order_id, 4.5, None).await },
        )
        .await;

        assert!(result.is_ok());
    }

    // send_jastiper_rating tanpa review (None) tetap berhasil
    #[tokio::test]
    async fn test_send_jastiper_rating_tanpa_review() {
        let server = MockServer::start().await;
        let jastiper_id = Uuid::new_v4();

        Mock::given(method("POST"))
            .and(path(format!("/internal/users/{}/rating", jastiper_id)))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let order_id = Uuid::new_v4();
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("USER_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { send_jastiper_rating(jastiper_id, order_id, 5.0, None).await },
        )
        .await;

        assert!(result.is_ok());
    }
}
