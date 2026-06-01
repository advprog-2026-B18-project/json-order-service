use std::sync::Arc;

use crate::infrastructure::publisher::MockCheckoutPublisher;
use crate::repositories::idempotency_repository::MockIdempotencyRepository;
use crate::repositories::order_repository::MockOrderRepository;
use crate::repositories::order_status_history_repository::MockOrderStatusHistoryRepository;
use crate::repositories::rating_jastiper_repository::MockRatingJastiperRepository;
use crate::repositories::rating_product_repository::MockRatingProductRepository;
use crate::services::auth_client::MockAuthClient;
use crate::services::inventory_client::MockInventoryClient;
use crate::services::wallet_client::MockWalletClient;
use crate::state::AppState;
use crate::tests::unit::controller::helper_test::dummy_mq_pool;

#[allow(clippy::too_many_arguments)]
pub fn make_state(
    order_repo: MockOrderRepository,
    inventory_client: MockInventoryClient,
    wallet_client: MockWalletClient,
    order_status_history_repo: MockOrderStatusHistoryRepository,
    rating_product_repo: MockRatingProductRepository,
    rating_jastiper_repo: MockRatingJastiperRepository,
    checkout_publisher: MockCheckoutPublisher,
    idempotency_repo: MockIdempotencyRepository,
) -> AppState {
    AppState {
        order_repo: Arc::new(order_repo),
        inventory_client: Arc::new(inventory_client),
        wallet_client: Arc::new(wallet_client),
        order_status_history_repo: Arc::new(order_status_history_repo),
        rating_product_repo: Arc::new(rating_product_repo),
        rating_jastiper_repo: Arc::new(rating_jastiper_repo),
        auth_client: Arc::new(MockAuthClient::new()),
        checkout_publisher: Arc::new(checkout_publisher),
        mq_pool: dummy_mq_pool(),
        idempotency_repo: Arc::new(idempotency_repo),
    }
}

pub fn setup_jwt_secret() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| unsafe {
        std::env::set_var("JWT_SECRET", "dGVzdC1zZWNyZXQtdGVzdC1zZWNyZXQ=");
    });
}

pub fn setup_internal_service_key() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| unsafe {
        std::env::set_var("INTERNAL_SERVICE_KEY", "internal-secret");
    });
}

pub fn json_request_no_auth(
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
) -> axum::http::Request<axum::body::Body> {
    let body_bytes = body.map(|v| v.to_string().into_bytes()).unwrap_or_default();
    axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(body_bytes))
        .expect("build request")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_request_no_auth_get_without_body() {
        let req = json_request_no_auth("GET", "/test", None);
        assert_eq!(req.method(), "GET");
        assert_eq!(req.uri(), "/test");
    }

    #[test]
    fn json_request_no_auth_post_with_body() {
        let req =
            json_request_no_auth("POST", "/orders", Some(serde_json::json!({"key": "value"})));
        assert_eq!(req.method(), "POST");
        assert_eq!(req.uri(), "/orders");
    }
}
