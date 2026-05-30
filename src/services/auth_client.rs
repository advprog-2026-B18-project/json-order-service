use crate::error::AppError;
use async_trait::async_trait;
use uuid::Uuid;

#[mockall::automock]
#[async_trait]
pub trait AuthClient: Send + Sync {
    async fn send_jastiper_rating<'a>(
        &self,
        jastiper_id: Uuid,
        order_id: Uuid,
        rating: f64,
        review: Option<&'a str>,
    ) -> Result<(), AppError>;

    async fn send_order_event(
        &self,
        jastiper_id: Uuid,
        event: &str,
    ) -> Result<(), AppError>;
}
