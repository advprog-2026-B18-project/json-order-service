use async_trait::async_trait;
use uuid::Uuid;

use crate::models::order_state::OrderStatus;
use crate::models::order_status_history::OrderStatusHistory;
use crate::models::role::Role;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait OrderStatusHistoryRepository: Send + Sync {
    async fn insert_status_history<'a>(
        &self,
        order_id: Uuid,
        status: &OrderStatus,
        changed_by: &str,
        actor_role: &Role,
        notes: Option<&'a str>,
    ) -> crate::error::Result<()>;

    async fn get_status_history(
        &self,
        order_id: Uuid,
    ) -> crate::error::Result<Vec<OrderStatusHistory>>;
}
