use async_trait::async_trait;
use uuid::Uuid;

use crate::models::rating_jastiper::{CreateRatingJastiperRequest, RatingJastiper};
use crate::ports::rating_jastiper_repository::RatingJastiperRepository;
use crate::repositories::rating_jastiper as rating_jastiper_repo;

#[derive(Clone)]
pub struct PgRatingJastiperRepository {
    pool: sqlx::PgPool,
}

impl PgRatingJastiperRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RatingJastiperRepository for PgRatingJastiperRepository {
    async fn find_by_id(
        &self,
        rating_jastiper_id: Uuid,
    ) -> crate::error::Result<Option<RatingJastiper>> {
        rating_jastiper_repo::find_by_id(&self.pool, rating_jastiper_id).await
    }

    async fn find_by_order_id(
        &self,
        order_id: Uuid,
    ) -> crate::error::Result<Option<RatingJastiper>> {
        rating_jastiper_repo::find_by_order_id(&self.pool, order_id).await
    }

    async fn create(
        &self,
        order_id: Uuid,
        titipers_id: Uuid,
        req: &CreateRatingJastiperRequest,
    ) -> crate::error::Result<RatingJastiper> {
        rating_jastiper_repo::create(&self.pool, order_id, titipers_id, req).await
    }
}
