use chrono::Utc;
use sea_query::{Expr, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::filter_pagination::PaginationParams;
use crate::models::order::OrderIden;
use crate::models::rating_jastiper::{
    CreateRatingJastiperRequest, RatingJastiper, RatingJastiperIden,
};

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
        .and_where(
            sea_query::Expr::col(RatingJastiperIden::RatingJastiperId).eq(rating_jastiper_id),
        )
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

pub async fn find_all_by_jastiper_id(
    pool: &PgPool,
    jastiper_id: Uuid,
    pagination: &PaginationParams,
) -> Result<(Vec<RatingJastiper>, i64)> {
    let final_limit = pagination.limit.unwrap_or(20).min(100);
    let offset = (pagination.page.unwrap_or(1).max(1) - 1) * final_limit;

    let mut subquery = Query::select();
    subquery
        .expr(Expr::col(OrderIden::OrderId))
        .from(OrderIden::Order)
        .and_where(Expr::col(OrderIden::JastiperId).eq(jastiper_id));

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
        .and_where(Expr::col(RatingJastiperIden::OrderId).in_subquery(subquery))
        .limit(final_limit as u64)
        .offset(offset as u64)
        .build_sqlx(PostgresQueryBuilder);

    let mut count_subquery = Query::select();
    count_subquery
        .expr(Expr::col(OrderIden::OrderId))
        .from(OrderIden::Order)
        .and_where(Expr::col(OrderIden::JastiperId).eq(jastiper_id));

    let (count_sql, count_values) = Query::select()
        .expr(Expr::col(RatingJastiperIden::RatingJastiperId).count())
        .from(RatingJastiperIden::RatingJastiper)
        .and_where(Expr::col(RatingJastiperIden::OrderId).in_subquery(count_subquery))
        .build_sqlx(PostgresQueryBuilder);

    let (rows_result, count_result) = tokio::join!(
        sqlx::query_as_with::<_, RatingJastiper, _>(&sql, values).fetch_all(pool),
        sqlx::query_scalar_with::<_, i64, _>(&count_sql, count_values).fetch_one(pool)
    );

    let rows = rows_result?;
    let total_count = count_result?;

    Ok((rows, total_count))
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

    find_by_id(pool, rating_jastiper_id)
        .await?
        .ok_or(AppError::Internal)
}
