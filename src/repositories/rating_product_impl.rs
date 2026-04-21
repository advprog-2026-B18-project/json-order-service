use async_trait::async_trait;
use uuid::Uuid;

use crate::models::rating_product::{CreateRatingProductRequest, RatingProduct};
use crate::ports::rating_product_repository::RatingProductRepository;
use crate::repositories::rating_product as rating_product_repo;

#[derive(Clone)]
pub struct PgRatingProductRepository {
    pool: sqlx::PgPool,
}

impl PgRatingProductRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RatingProductRepository for PgRatingProductRepository {
    async fn find_by_id(
        &self,
        rating_product_id: Uuid,
    ) -> crate::error::Result<Option<RatingProduct>> {
        rating_product_repo::find_by_id(&self.pool, rating_product_id).await
    }

    async fn find_by_order_id(
        &self,
        order_id: Uuid,
    ) -> crate::error::Result<Option<RatingProduct>> {
        rating_product_repo::find_by_order_id(&self.pool, order_id).await
    }

    async fn create(
        &self,
        order_id: Uuid,
        titipers_id: Uuid,
        req: &CreateRatingProductRequest,
    ) -> crate::error::Result<RatingProduct> {
        rating_product_repo::create(&self.pool, order_id, titipers_id, req).await
    }
}
