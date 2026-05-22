use async_trait::async_trait;
use uuid::Uuid;

use crate::error::Result;
use crate::models::filter_pagination::PaginationParams;
use crate::models::rating_jastiper::{CreateRatingJastiperRequest, RatingJastiper};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait RatingJastiperRepository: Send + Sync {
    async fn find_by_id(&self, rating_jastiper_id: Uuid) -> Result<Option<RatingJastiper>>;
    async fn find_by_order_id(&self, order_id: Uuid) -> Result<Option<RatingJastiper>>;

    async fn find_all_by_jastiper_id(
        &self,
        jastiper_id: Uuid,
        pagination: &PaginationParams,
    ) -> Result<(Vec<RatingJastiper>, i64)>;

    async fn create(
        &self,
        order_id: Uuid,
        titipers_id: Uuid,
        req: &CreateRatingJastiperRequest,
    ) -> Result<RatingJastiper>;
}
