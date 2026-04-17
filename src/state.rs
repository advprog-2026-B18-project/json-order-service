use crate::ports::auth_client::AuthClient;
use crate::ports::inventory_client::InventoryClient;
use crate::ports::order_repository::OrderRepository;
use crate::ports::order_status_history_repository::OrderStatusHistoryRepository;
use crate::ports::rating_jastiper_repository::RatingJastiperRepository;
use crate::ports::rating_product_repository::RatingProductRepository;
use crate::ports::wallet_client::WalletClient;
use std::sync::Arc;

pub struct AppState {
    pub order_repo: Arc<dyn OrderRepository + Send + Sync>,
    pub order_status_history_repo: Arc<dyn OrderStatusHistoryRepository + Send + Sync>,
    pub rating_product_repo: Arc<dyn RatingProductRepository + Send + Sync>,
    pub rating_jastiper_repo: Arc<dyn RatingJastiperRepository + Send + Sync>,
    pub inventory_client: Arc<dyn InventoryClient + Send + Sync>,
    pub wallet_client: Arc<dyn WalletClient + Send + Sync>,
    pub auth_client: Arc<dyn AuthClient + Send + Sync>,
}
