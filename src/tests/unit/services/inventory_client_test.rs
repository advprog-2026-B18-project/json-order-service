#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::services::inventory_client::{
        fetch_product, release_stock, reserve_stock, send_product_rating,
    };

    // reserve_stock berhasil (200)
    #[tokio::test]
    async fn test_reserve_stock_berhasil() {
        let server = MockServer::start().await;
        let product_id = Uuid::new_v4();

        Mock::given(method("POST"))
            .and(path(format!(
                "/internal/products/{}/stock/reserve",
                product_id
            )))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let order_id = Uuid::new_v4();
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("INVENTORY_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { reserve_stock(product_id, order_id, 2).await },
        )
            .await;

        assert!(result.is_ok());
    }

    // reserve_stock stok tidak cukup (409) → Conflict
    #[tokio::test]
    async fn test_reserve_stock_tidak_cukup() {
        let server = MockServer::start().await;
        let product_id = Uuid::new_v4();

        Mock::given(method("POST"))
            .and(path(format!(
                "/internal/products/{}/stock/reserve",
                product_id
            )))
            .respond_with(ResponseTemplate::new(409))
            .mount(&server)
            .await;

        let order_id = Uuid::new_v4();
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("INVENTORY_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { reserve_stock(product_id, order_id, 2).await },
        )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::AppError::Conflict(_) => {}
            e => panic!("Expected Conflict, got {:?}", e),
        }
    }

    // reserve_stock produk tidak ditemukan (404) → NotFound
    #[tokio::test]
    async fn test_reserve_stock_produk_tidak_ditemukan() {
        let server = MockServer::start().await;
        let product_id = Uuid::new_v4();

        Mock::given(method("POST"))
            .and(path(format!(
                "/internal/products/{}/stock/reserve",
                product_id
            )))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let order_id = Uuid::new_v4();
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("INVENTORY_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { reserve_stock(product_id, order_id, 2).await },
        )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::AppError::NotFound(_) => {}
            e => panic!("Expected NotFound, got {:?}", e),
        }
    }

    // reserve_stock produk tidak ACTIVE (422) → UnprocessableEntity
    #[tokio::test]
    async fn test_reserve_stock_produk_tidak_active() {
        let server = MockServer::start().await;
        let product_id = Uuid::new_v4();

        Mock::given(method("POST"))
            .and(path(format!(
                "/internal/products/{}/stock/reserve",
                product_id
            )))
            .respond_with(ResponseTemplate::new(422))
            .mount(&server)
            .await;

        let order_id = Uuid::new_v4();
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("INVENTORY_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { reserve_stock(product_id, order_id, 2).await },
        )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::AppError::UnprocessableEntity(_) => {}
            e => panic!("Expected UnprocessableEntity, got {:?}", e),
        }
    }

    // release_stock berhasil (200)
    #[tokio::test]
    async fn test_release_stock_berhasil() {
        let server = MockServer::start().await;
        let product_id = Uuid::new_v4();

        Mock::given(method("POST"))
            .and(path(format!(
                "/internal/products/{}/stock/release",
                product_id
            )))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let order_id = Uuid::new_v4();
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("INVENTORY_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { release_stock(product_id, order_id, 2).await },
        )
            .await;

        assert!(result.is_ok());
    }

    // fetch_product berhasil (200) → return JSON data
    #[tokio::test]
    async fn test_fetch_product_berhasil() {
        let server = MockServer::start().await;
        let product_id = Uuid::new_v4();

        Mock::given(method("GET"))
            .and(path(format!("/products/{}", product_id)))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "data": {
                        "id": product_id,
                        "name": "Matcha Kit Kat",
                        "price": 25000,
                        "jastiperId": Uuid::new_v4(),
                    }
                })),
            )
            .mount(&server)
            .await;

        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("INVENTORY_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { fetch_product(product_id).await },
        )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap()["name"], "Matcha Kit Kat");
    }

    // fetch_product tidak ditemukan (404) → NotFound
    #[tokio::test]
    async fn test_fetch_product_tidak_ditemukan() {
        let server = MockServer::start().await;
        let product_id = Uuid::new_v4();

        Mock::given(method("GET"))
            .and(path(format!("/products/{}", product_id)))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("INVENTORY_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { fetch_product(product_id).await },
        )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::AppError::NotFound(_) => {}
            e => panic!("Expected NotFound, got {:?}", e),
        }
    }

    // send_product_rating berhasil (200)
    #[tokio::test]
    async fn test_send_product_rating_berhasil() {
        let server = MockServer::start().await;
        let product_id = Uuid::new_v4();

        Mock::given(method("POST"))
            .and(path(format!(
                "/internal/products/{}/post-order",
                product_id
            )))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let order_id = Uuid::new_v4();
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("INVENTORY_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move {
                send_product_rating(product_id, order_id, 4.5, Some("Bagus"), vec![]).await
            },
        )
            .await;

        assert!(result.is_ok());
    }

    // send_product_rating non-fatal: 404 → Ok
    #[tokio::test]
    async fn test_send_product_rating_404_nonfatal() {
        let server = MockServer::start().await;
        let product_id = Uuid::new_v4();

        Mock::given(method("POST"))
            .and(path(format!(
                "/internal/products/{}/post-order",
                product_id
            )))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let order_id = Uuid::new_v4();
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("INVENTORY_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { send_product_rating(product_id, order_id, 4.5, None, vec![]).await },
        )
            .await;

        assert!(result.is_ok()); // non-fatal
    }

    // send_product_rating idempotent (409) → Ok
    #[tokio::test]
    async fn test_send_product_rating_idempotent() {
        let server = MockServer::start().await;
        let product_id = Uuid::new_v4();

        Mock::given(method("POST"))
            .and(path(format!(
                "/internal/products/{}/post-order",
                product_id
            )))
            .respond_with(ResponseTemplate::new(409))
            .mount(&server)
            .await;

        let order_id = Uuid::new_v4();
        let uri = server.uri();
        let result = temp_env::async_with_vars(
            [
                ("INVENTORY_SERVICE_URL", Some(uri.as_str())),
                ("INTERNAL_SERVICE_KEY", Some("test-key")),
            ],
            async move { send_product_rating(product_id, order_id, 4.5, None, vec![]).await },
        )
            .await;

        assert!(result.is_ok());
    }
}