use crate::error::AppError;
use crate::services::auth_client::AuthClient;
use crate::services::implements::auth_client_impl::send_jastiper_rating;
use async_trait::async_trait;
use uuid::Uuid;

pub struct HttpAuthClient;

#[async_trait]
impl AuthClient for HttpAuthClient {
    async fn send_jastiper_rating<'a>(
        &self,
        jastiper_id: Uuid,
        order_id: Uuid,
        rating: f64,
        review: Option<&'a str>,
    ) -> Result<(), AppError> {
        send_jastiper_rating(jastiper_id, order_id, rating, review).await
    }
}

