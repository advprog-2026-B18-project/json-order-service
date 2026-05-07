use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::filter_pagination::{OrderFilter, PaginationParams};
use crate::models::order::{CreateOrderRequest, Order, PriceBreakdown, UpdateOrderParams};
use crate::models::order_state::OrderStatus;
use crate::repositories::implements::order_repo_impl as order_repo;
use crate::repositories::order_repository::OrderRepository;
use crate::repositories::order_status_history_repository::OrderStatusHistoryRepository;

#[derive(Clone)]
pub struct PgOrderRepository {
    pool: sqlx::PgPool,
    order_status_history_repo: Arc<dyn OrderStatusHistoryRepository + Send + Sync>,
}

impl PgOrderRepository {
    pub fn new(
        pool: sqlx::PgPool,
        order_status_history_repo: Arc<dyn OrderStatusHistoryRepository + Send + Sync>,
    ) -> Self {
        Self {
            pool,
            order_status_history_repo,
        }
    }
}

#[async_trait]
impl OrderRepository for PgOrderRepository {
    async fn find_all<'a>(
        &self,
        filter: Option<&'a OrderFilter>,
        pagination: &PaginationParams,
    ) -> crate::error::Result<(Vec<Order>, i64)> {
        order_repo::find_all(&self.pool, filter, pagination).await
    }

    async fn find_by_id(&self, order_id: Uuid) -> crate::error::Result<Option<Order>> {
        order_repo::find_by_id(&self.pool, order_id).await
    }

    async fn create(
        &self,
        titipers_id: Uuid,
        jastiper_id: Uuid,
        req: CreateOrderRequest,
        product_snapshot: Value,
        price: PriceBreakdown,
    ) -> crate::error::Result<Order> {
        order_repo::create(
            &self.pool,
            &*self.order_status_history_repo,
            titipers_id,
            jastiper_id,
            req,
            product_snapshot,
            price,
        )
        .await
    }

    async fn update<'a>(
        &self,
        order_id: Uuid,
        new_status: &OrderStatus,
        params: UpdateOrderParams<'a>,
    ) -> crate::error::Result<Order> {
        order_repo::update(
            &self.pool,
            &*self.order_status_history_repo,
            order_id,
            new_status,
            params,
        )
        .await
    }

    async fn delete(&self, order_id: Uuid) -> crate::error::Result<()> {
        order_repo::delete(&self.pool, order_id).await
    }
}
