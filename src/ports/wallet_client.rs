use crate::error::AppError;
use async_trait::async_trait;
use uuid::Uuid;

#[mockall::automock]
#[async_trait]
pub trait WalletClient: Send + Sync {
    async fn deduct_wallet(
        &self,
        user_id: Uuid,
        order_id: Uuid,
        amount: i64,
        description: &str,
    ) -> Result<(), AppError>;

    async fn refund_wallet(
        &self,
        user_id: Uuid,
        order_id: Uuid,
        amount: i64,
        description: &str,
    ) -> Result<(), AppError>;

    async fn check_wallet(&self, user_id: Uuid, req_amount: i64) -> Result<(), AppError>;
}
