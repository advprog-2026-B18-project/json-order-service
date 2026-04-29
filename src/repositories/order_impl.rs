use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::models::filter_pagination::{OrderFilter, PaginationParams};
use crate::models::order::{CreateOrderRequest, Order, PriceBreakdown, UpdateOrderParams};
use crate::models::order_state::OrderStatus;
pub(crate) use crate::ports::order_repository::OrderRepository;
use crate::repositories::order as order_repo;

#[derive(Clone)]
pub struct PgOrderRepository {
    pool: sqlx::PgPool,
}

impl PgOrderRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
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
        order_repo::find_by_id(&self.pool, order_id).await // ← pool dari self
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
        order_repo::update(&self.pool, order_id, new_status, params).await
    }
}
