use crate::error::AppError;
use crate::services::implements::wallet_client_impl::{
    check_wallet, deduct_wallet, earnings_wallet, refund_wallet, reverse_earnings,
};
use crate::services::wallet_client::{
    DeductResponse, EarningsResponse, RefundResponse, WalletClient,
};
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
    ) -> Result<DeductResponse, AppError> {
        deduct_wallet(user_id, order_id, amount, description).await
    }

    async fn refund_wallet(
        &self,
        user_id: Uuid,
        order_id: Uuid,
        amount: i64,
        description: &str,
    ) -> Result<RefundResponse, AppError> {
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
    ) -> Result<EarningsResponse, AppError> {
        earnings_wallet(jastiper_id, order_id, description).await
    }

    async fn reverse_earnings(
        &self,
        jastiper_id: Uuid,
        order_id: Uuid,
        transaction_id: &str,
        description: &str,
    ) -> Result<(), AppError> {
        reverse_earnings(jastiper_id, order_id, transaction_id, description).await
    }
}
