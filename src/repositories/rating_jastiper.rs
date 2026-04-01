use sea_query::{PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;

use crate::error::{AppError, Result};
use crate::models::rating_jastiper::{RatingJastiperIden, RatingJastiper, CreateRatingJastiperRequest};

pub async fn find_by_id(pool: &PgPool, rating_jastiper_id: Uuid) -> Result<Option<RatingJastiper>> {
    let (sql, values) = Query::select()
        .columns([
            RatingJastiperIden::RatingJastiperId,
            RatingJastiperIden::OrderId,
            RatingJastiperIden::TitipersId,
            RatingJastiperIden::JastiperRating,
            RatingJastiperIden::JastiperReview,
            RatingJastiperIden::CreatedAt,
        ])
        .from(RatingJastiperIden::RatingJastiper)
        .and_where(sea_query::Expr::col(RatingJastiperIden::RatingJastiperId).eq(rating_jastiper_id))
        .build_sqlx(PostgresQueryBuilder);

    let row = sqlx::query_as_with::<_, RatingJastiper, _>(&sql, values)
        .fetch_optional(pool)
        .await?;

    Ok(row)
}

pub async fn find_by_order_id(pool: &PgPool, order_id: Uuid) -> Result<Option<RatingJastiper>> {
    let (sql, values) = Query::select()
        .columns([
            RatingJastiperIden::RatingJastiperId,
            RatingJastiperIden::OrderId,
            RatingJastiperIden::TitipersId,
            RatingJastiperIden::JastiperRating,
            RatingJastiperIden::JastiperReview,
            RatingJastiperIden::CreatedAt,
        ])
        .from(RatingJastiperIden::RatingJastiper)
        .and_where(sea_query::Expr::col(RatingJastiperIden::OrderId).eq(order_id))
        .build_sqlx(PostgresQueryBuilder);

    let row = sqlx::query_as_with::<_, RatingJastiper, _>(&sql, values)
        .fetch_optional(pool)
        .await?;

    Ok(row)
}

pub async fn create(
    pool: &PgPool,
    order_id: Uuid,
    titipers_id: Uuid,
    req: &CreateRatingJastiperRequest,
) -> Result<RatingJastiper> {
    let rating_jastiper_id = Uuid::new_v4();
    let now = Utc::now();

    let (sql, values) = Query::insert()
        .into_table(RatingJastiperIden::RatingJastiper)
        .columns([
            RatingJastiperIden::RatingJastiperId,
            RatingJastiperIden::OrderId,
            RatingJastiperIden::TitipersId,
            RatingJastiperIden::JastiperRating,
            RatingJastiperIden::JastiperReview,
            RatingJastiperIden::CreatedAt,
        ])
        .values_panic([
            rating_jastiper_id.into(),
            order_id.into(),
            titipers_id.into(),
            req.jastiper_rating.into(),
            req.jastiper_review.clone().unwrap_or_default().into(),
            now.into(),
        ])
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_with(&sql, values).execute(pool).await?;

    find_by_id(pool, rating_jastiper_id).await?.ok_or(AppError::Internal)
}