use crate::error::AppError;
use crate::ports::wallet_client::WalletClient;
use crate::services::wallet_client::{check_wallet, deduct_wallet, earnings_wallet, refund_wallet};
use async_trait::async_trait;
use uuid::Uuid;

pub struct HttpWalletClient;

#[async_trait]
impl WalletClient for HttpWalletClient {
    async fn deduct_wallet(
        &self,
        user_id: Uuid,
        order_id: Uuid,
        amount: i64,
        description: &str,
    ) -> Result<(), AppError> {
        deduct_wallet(user_id, order_id, amount, description).await
    }

    async fn refund_wallet(
        &self,
        user_id: Uuid,
        order_id: Uuid,
        amount: i64,
        description: &str,
    ) -> Result<(), AppError> {
        refund_wallet(user_id, order_id, amount, description).await
    }

    async fn check_wallet(&self, user_id: Uuid, req_amount: i64) -> Result<(), AppError> {
        check_wallet(user_id, req_amount).await
    }

    async fn earnings_wallet(
        &self,
        jastiper_id: Uuid,
        order_id: Uuid,
        description: &str,
    ) -> Result<(), AppError> {
        earnings_wallet(jastiper_id, order_id, description).await
    }
}
