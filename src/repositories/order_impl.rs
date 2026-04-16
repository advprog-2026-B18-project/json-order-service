use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::models::filter_pagination::{OrderFilter, PaginationParams};
use crate::models::order::{CreateOrderRequest, Order};
use crate::models::order_state::OrderStatus;
use crate::models::role::Role;
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
        unit_price: i64,
        service_fee: i64,
        total_price: i64,
    ) -> crate::error::Result<Order> {
        order_repo::create(
            &self.pool,
            titipers_id,
            jastiper_id,
            req,
            product_snapshot,
            unit_price,
            service_fee,
            total_price,
        )
        .await
    }

    async fn update<'a>(
        &self,
        order_id: Uuid,
        new_status: &OrderStatus,
        changed_by: &str,
        actor_role: &Role,
        notes: Option<&'a str>,
        tracking_number: Option<&'a str>,
        courier: Option<&'a str>,
        cancellation_reason: Option<&'a str>,
    ) -> crate::error::Result<Order> {
        order_repo::update(
            &self.pool,
            order_id,
            new_status,
            changed_by,
            actor_role,
            notes,
            tracking_number,
            courier,
            cancellation_reason,
        )
        .await
    }
}
