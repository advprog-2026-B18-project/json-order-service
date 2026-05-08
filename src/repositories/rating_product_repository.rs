use async_trait::async_trait;
use uuid::Uuid;

use crate::error::Result;
use crate::models::rating_product::{CreateRatingProductRequest, RatingProduct};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait RatingProductRepository: Send + Sync {
    async fn find_by_id(&self, rating_product_id: Uuid) -> Result<Option<RatingProduct>>;
    async fn find_by_order_id(&self, order_id: Uuid) -> Result<Option<RatingProduct>>;

    async fn create(
        &self,
        order_id: Uuid,
        titipers_id: Uuid,
        req: &CreateRatingProductRequest,
    ) -> Result<RatingProduct>;
}
