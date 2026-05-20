use crate::error::AppError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait IdempotencyRepository: Send + Sync {
    async fn is_processed(&self, key: Uuid) -> Result<bool, AppError>;
    async fn mark_processed(&self, key: Uuid, order_id: Uuid) -> Result<(), AppError>;
}
