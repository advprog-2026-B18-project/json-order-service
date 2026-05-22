use chrono::Utc;
use sea_query::{Expr, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::filter_pagination::PaginationParams;
use crate::models::order::OrderIden;
use crate::models::rating_product::{CreateRatingProductRequest, RatingProduct, RatingProductIden};

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

pub async fn find_all_by_product_id(
    pool: &PgPool,
    product_id: Uuid,
    pagination: &PaginationParams,
) -> Result<(Vec<RatingProduct>, i64)> {
    let final_limit = pagination.limit.unwrap_or(20).min(100);
    let offset = (pagination.page.unwrap_or(1).max(1) - 1) * final_limit;

    let mut subquery = Query::select();
    subquery
        .expr(Expr::col(OrderIden::OrderId))
        .from(OrderIden::Order)
        .and_where(Expr::col(OrderIden::ProductId).eq(product_id));

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
        .and_where(Expr::col(RatingProductIden::OrderId).in_subquery(subquery))
        .limit(final_limit as u64)
        .offset(offset as u64)
        .build_sqlx(PostgresQueryBuilder);

    let mut count_subquery = Query::select();
    count_subquery
        .expr(Expr::col(OrderIden::OrderId))
        .from(OrderIden::Order)
        .and_where(Expr::col(OrderIden::ProductId).eq(product_id));

    let (count_sql, count_values) = Query::select()
        .expr(Expr::col(RatingProductIden::RatingProductId).count())
        .from(RatingProductIden::RatingProduct)
        .and_where(Expr::col(RatingProductIden::OrderId).in_subquery(count_subquery))
        .build_sqlx(PostgresQueryBuilder);

    let (rows_result, count_result) = tokio::join!(
        sqlx::query_as_with::<_, RatingProduct, _>(&sql, values).fetch_all(pool),
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
            Expr::cust(format!(
                "ARRAY[{}]::TEXT[]",
                images
                    .iter()
                    .map(|s| format!("'{}'", s.replace("'", "''")))
                    .collect::<Vec<_>>()
                    .join(",")
            )),
            now.into(),
        ])
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_with(&sql, values).execute(pool).await?;

    find_by_id(pool, rating_product_id)
        .await?
        .ok_or(AppError::Internal)
}
