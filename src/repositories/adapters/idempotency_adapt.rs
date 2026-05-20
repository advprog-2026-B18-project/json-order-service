use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::repositories::idempotency_repository::IdempotencyRepository;

pub struct PgIdempotencyRepository {
    pool: PgPool,
}

impl PgIdempotencyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IdempotencyRepository for PgIdempotencyRepository {
    async fn is_processed(&self, key: Uuid) -> Result<bool, AppError> {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM idempotency_keys WHERE key = $1)",
            key
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| AppError::Internal)?
        .unwrap_or(false);

        Ok(exists)
    }

    async fn mark_processed(&self, key: Uuid, order_id: Uuid) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO idempotency_keys (key, order_id)
            VALUES ($1, $2)
            ON CONFLICT (key) DO NOTHING
            "#,
            key,
            order_id,
        )
        .execute(&self.pool)
        .await
        .map_err(|_| AppError::Internal)?;

        Ok(())
    }
}
