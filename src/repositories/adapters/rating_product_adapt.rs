use async_trait::async_trait;
use uuid::Uuid;

use crate::models::filter_pagination::PaginationParams;
use crate::models::rating_product::{CreateRatingProductRequest, RatingProduct};
use crate::repositories::implements::rating_product_repo_impl as rating_product_repo;
use crate::repositories::rating_product_repository::RatingProductRepository;

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

    async fn find_all_by_product_id(
        &self,
        product_id: Uuid,
        pagination: &PaginationParams,
    ) -> crate::error::Result<(Vec<RatingProduct>, i64)> {
        rating_product_repo::find_all_by_product_id(&self.pool, product_id, pagination).await
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
