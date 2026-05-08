use crate::repositories::order_repository::OrderRepository;
use crate::repositories::order_status_history_repository::OrderStatusHistoryRepository;
use crate::repositories::rating_jastiper_repository::RatingJastiperRepository;
use crate::repositories::rating_product_repository::RatingProductRepository;
use crate::services::auth_client::AuthClient;
use crate::services::inventory_client::InventoryClient;
use crate::services::wallet_client::WalletClient;
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
