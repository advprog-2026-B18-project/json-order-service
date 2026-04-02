use sea_query::{PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;

use crate::error::{AppError, Result};
use crate::models::rating_product::{RatingProductIden, RatingProduct, CreateRatingProductRequest};

pub async fn find_by_id(pool: &PgPool, rating_product_id: Uuid) -> Result<Option<RatingProduct>> {
    let (sql, values) = Query::select()
        .columns([
            RatingProductIden::RatingProductId,
            RatingProductIden::OrderId,
            RatingProductIden::TitipersId,
            RatingProductIden::ProductRating,
            RatingProductIden::ProductReview,
            RatingProductIden::ProductImages,
            RatingProductIden::CreatedAt,
        ])
        .from(RatingProductIden::RatingProduct)
        .and_where(sea_query::Expr::col(RatingProductIden::RatingProductId).eq(rating_product_id))
        .build_sqlx(PostgresQueryBuilder);

    let row = sqlx::query_as_with::<_, RatingProduct, _>(&sql, values)
        .fetch_optional(pool)
        .await?;

    Ok(row)
}

pub async fn find_by_order_id(pool: &PgPool, order_id: Uuid) -> Result<Option<RatingProduct>> {
    let (sql, values) = Query::select()
        .columns([
            RatingProductIden::RatingProductId,
            RatingProductIden::OrderId,
            RatingProductIden::TitipersId,
            RatingProductIden::ProductRating,
            RatingProductIden::ProductReview,
            RatingProductIden::ProductImages,
            RatingProductIden::CreatedAt,
        ])
        .from(RatingProductIden::RatingProduct)
        .and_where(sea_query::Expr::col(RatingProductIden::OrderId).eq(order_id))
        .build_sqlx(PostgresQueryBuilder);

    let row = sqlx::query_as_with::<_, RatingProduct, _>(&sql, values)
        .fetch_optional(pool)
        .await?;

    Ok(row)
}

pub async fn create(
    pool: &PgPool,
    order_id: Uuid,
    titipers_id: Uuid,
    req: &CreateRatingProductRequest,
) -> Result<RatingProduct> {
    let rating_product_id = Uuid::new_v4();
    let now = Utc::now();
    let images = req.product_images.clone().unwrap_or_default();

    let (sql, values) = Query::insert()
        .into_table(RatingProductIden::RatingProduct)
        .columns([
            RatingProductIden::RatingProductId,
            RatingProductIden::OrderId,
            RatingProductIden::TitipersId,
            RatingProductIden::ProductRating,
            RatingProductIden::ProductReview,
            RatingProductIden::ProductImages,
            RatingProductIden::CreatedAt,
        ])
        .values_panic([
            rating_product_id.into(),
            order_id.into(),
            titipers_id.into(),
            req.product_rating.into(),
            req.product_review.clone().unwrap_or_default().into(),
            serde_json::to_value(&images).unwrap().into(),
            now.into(),
        ])
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_with(&sql, values).execute(pool).await?;

    find_by_id(pool, rating_product_id).await?.ok_or(AppError::Internal)
}