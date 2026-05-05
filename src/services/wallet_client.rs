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
    ) -> Result<DeductResponse, AppError>;

    async fn refund_wallet(
        &self,
        user_id: Uuid,
        order_id: Uuid,
        amount: i64,
        description: &str,
    ) -> Result<RefundResponse, AppError>;

    async fn check_wallet(&self, user_id: Uuid, req_amount: i64) -> Result<(), AppError>;

    async fn earnings_wallet(
        &self,
        jastiper_id: Uuid,
        order_id: Uuid,
        description: &str,
    ) -> Result<EarningsResponse, AppError>;

    async fn reverse_earnings(
        &self,
        jastiper_id: Uuid,
        order_id: Uuid,
        transaction_id: &str,
        description: &str,
    ) -> Result<(), AppError>;
}

#[derive(Debug)]
pub struct EarningsResponse {
    pub transaction_id: String,
}

#[derive(Debug)]
pub struct DeductResponse {
    pub transaction_id: String,
}

#[derive(Debug)]
pub struct RefundResponse {
    pub transaction_id: String,
}
