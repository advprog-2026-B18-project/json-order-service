use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::error::Result;
use crate::models::filter_pagination::{OrderFilter, PaginationParams};
use crate::models::order::{CreateOrderRequest, Order};
use crate::models::order::{PriceBreakdown, UpdateOrderParams};
use crate::models::order_state::OrderStatus;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait OrderRepository: Send + Sync {
    async fn find_all<'a>(
        &self,
        filter: Option<&'a OrderFilter>,
        pagination: &PaginationParams,
    ) -> Result<(Vec<Order>, i64)>;

    async fn find_by_id(&self, order_id: Uuid) -> Result<Option<Order>>;

    async fn create(
        &self,
        order_id: Uuid,
        titipers_id: Uuid,
        jastiper_id: Uuid,
        req: CreateOrderRequest,
        product_snapshot: Value,
        price: PriceBreakdown,
    ) -> Result<Order>;

    async fn update<'a>(
        &self,
        order_id: Uuid,
        new_status: &OrderStatus,
        params: UpdateOrderParams<'a>,
    ) -> Result<Order>;

    async fn delete(&self, order_id: Uuid) -> Result<()>;

    async fn find_expired_pending_orders(&self) -> Result<Vec<Order>>;
}
