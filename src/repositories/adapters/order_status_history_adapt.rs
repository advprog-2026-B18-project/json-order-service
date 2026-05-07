use async_trait::async_trait;
use uuid::Uuid;

use crate::models::order_state::OrderStatus;
use crate::models::order_status_history::OrderStatusHistory;
use crate::models::role::Role;
use crate::repositories::implements::order_status_history_repo_impl as order_status_history_repo;
use crate::repositories::order_status_history_repository::OrderStatusHistoryRepository;

#[derive(Clone)]
pub struct PgOrderStatusHistoryRepository {
    pool: sqlx::PgPool,
}

impl PgOrderStatusHistoryRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OrderStatusHistoryRepository for PgOrderStatusHistoryRepository {
    async fn insert_status_history<'a>(
        &self,
        order_id: Uuid,
        status: &OrderStatus,
        changed_by: &str,
        actor_role: &Role,
        notes: Option<&'a str>,
    ) -> crate::error::Result<()> {
        order_status_history_repo::insert_status_history(
            &self.pool, order_id, status, changed_by, actor_role, notes,
        )
        .await
    }

    async fn get_status_history(
        &self,
        order_id: Uuid,
    ) -> crate::error::Result<Vec<OrderStatusHistory>> {
        order_status_history_repo::get_status_history(&self.pool, order_id).await
    }
}
