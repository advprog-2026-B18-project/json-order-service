use chrono::Utc;
use sea_query::Order::{Asc, Desc};
use sea_query::{Alias, Cond, Expr, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::filter_pagination::{OrderFilter, PaginationParams, SortOrder};
pub(crate) use crate::models::order::{CreateOrderRequest, Order, OrderIden};
use crate::models::order::{PriceBreakdown, UpdateOrderParams};
use crate::models::order_state::OrderStatus;
use crate::models::role::Role;
use crate::repositories::order_status_history_repository::OrderStatusHistoryRepository;

pub async fn find_all(
    pool: &PgPool,
    filter: Option<&OrderFilter>,
    pagination: &PaginationParams,
) -> Result<(Vec<Order>, i64)> {
    let final_limit = pagination.limit.unwrap_or(20).min(100);
    let offset = (pagination.page.unwrap_or(1).max(1) - 1) * final_limit;

    let sort_order = match pagination.order.as_ref().unwrap_or(&SortOrder::Asc) {
        SortOrder::Desc => Desc,
        _ => Asc,
    };

    let sort_col = match pagination.sort_by.as_deref() {
        Some("created_at") => OrderIden::CreatedAt,
        Some("updated_at") => OrderIden::UpdatedAt,
        Some("total_price") => OrderIden::TotalPrice,
        _ => OrderIden::CreatedAt,
    };

    let condition = build_filter_condition(filter);

    let (sql, values) = Query::select()
        .columns([
            OrderIden::OrderId,
            OrderIden::TitipersId,
            OrderIden::JastiperId,
            OrderIden::ProductId,
            OrderIden::ProductSnapshot,
            OrderIden::Quantity,
            OrderIden::UnitPrice,
            OrderIden::ServiceFee,
            OrderIden::TotalPrice,
            OrderIden::Status,
            OrderIden::ShippingAddress,
            OrderIden::NoteToJastiper,
            OrderIden::TrackingNumber,
            OrderIden::Courier,
            OrderIden::CancellationReason,
            OrderIden::CancelledBy,
            OrderIden::CompletedAt,
            OrderIden::CreatedAt,
            OrderIden::UpdatedAt,
            OrderIden::ExpiredAt,
        ])
        .from(OrderIden::Order)
        .cond_where(condition.clone())
        .order_by(sort_col, sort_order)
        .limit(final_limit as u64)
        .offset(offset as u64)
        .build_sqlx(PostgresQueryBuilder);

    let (count_sql, count_values) = Query::select()
        .expr(Expr::col(OrderIden::OrderId).count())
        .from(OrderIden::Order)
        .cond_where(condition)
        .build_sqlx(PostgresQueryBuilder);

    let (orders_result, count_result) = tokio::join!(
        sqlx::query_as_with::<_, Order, _>(&sql, values).fetch_all(pool),
        sqlx::query_scalar_with::<_, i64, _>(&count_sql, count_values).fetch_one(pool)
    );

    let orders = orders_result?;
    let total_count = count_result?;

    Ok((orders, total_count))
}

fn build_filter_condition(filter: Option<&OrderFilter>) -> Cond {
    let mut cond = Cond::all();

    let Some(f) = filter else {
        return cond;
    };

    if let Some(id) = f.titipers_id {
        cond = cond.add(Expr::col(OrderIden::TitipersId).eq(id));
    }
    if let Some(id) = f.jastiper_id {
        cond = cond.add(Expr::col(OrderIden::JastiperId).eq(id));
    }
    if let Some(id) = f.product_id {
        cond = cond.add(Expr::col(OrderIden::ProductId).eq(id));
    }

    if let Some(ref status) = f.status {
        cond = cond.add(
            Expr::col(OrderIden::Status)
                .cast_as(Alias::new("TEXT"))
                .eq(status.to_string()),
        );
    }

    if let Some(date_from) = f.date_from {
        cond = cond.add(Expr::col(OrderIden::CreatedAt).gte(date_from));
    }
    if let Some(date_to) = f.date_to {
        cond = cond.add(Expr::col(OrderIden::CreatedAt).lte(date_to));
    }

    cond
}

pub async fn find_by_id(pool: &PgPool, order_id: Uuid) -> Result<Option<Order>> {
    let (sql, values) = Query::select()
        .columns([
            OrderIden::OrderId,
            OrderIden::TitipersId,
            OrderIden::JastiperId,
            OrderIden::ProductId,
            OrderIden::ProductSnapshot,
            OrderIden::Quantity,
            OrderIden::UnitPrice,
            OrderIden::ServiceFee,
            OrderIden::TotalPrice,
            OrderIden::Status,
            OrderIden::ShippingAddress,
            OrderIden::NoteToJastiper,
            OrderIden::TrackingNumber,
            OrderIden::Courier,
            OrderIden::CancellationReason,
            OrderIden::CancelledBy,
            OrderIden::CompletedAt,
            OrderIden::CreatedAt,
            OrderIden::UpdatedAt,
            OrderIden::ExpiredAt,
        ])
        .from(OrderIden::Order)
        .and_where(Expr::col(OrderIden::OrderId).eq(order_id))
        .build_sqlx(PostgresQueryBuilder);

    let order = sqlx::query_as_with::<_, Order, _>(&sql, values)
        .fetch_optional(pool)
        .await?;

    Ok(order)
}

pub async fn create(
    pool: &PgPool,
    order_status_history_repo: &(dyn OrderStatusHistoryRepository + Send + Sync),
    titipers_id: Uuid,
    jastiper_id: Uuid,
    req: CreateOrderRequest,
    product_snapshot: serde_json::Value,
    price: PriceBreakdown,
) -> Result<Order> {
    let order_id = Uuid::new_v4();
    let now = Utc::now();
    let expired_at = now + chrono::Duration::minutes(15);

    let (sql, values) = Query::insert()
        .into_table(OrderIden::Order)
        .columns([
            OrderIden::OrderId,
            OrderIden::TitipersId,
            OrderIden::JastiperId,
            OrderIden::ProductId,
            OrderIden::ProductSnapshot,
            OrderIden::Quantity,
            OrderIden::UnitPrice,
            OrderIden::ServiceFee,
            OrderIden::TotalPrice,
            OrderIden::Status,
            OrderIden::ShippingAddress,
            OrderIden::NoteToJastiper,
            OrderIden::CreatedAt,
            OrderIden::UpdatedAt,
            OrderIden::ExpiredAt,
        ])
        .values_panic([
            order_id.into(),
            titipers_id.into(),
            jastiper_id.into(),
            req.product_id.into(),
            product_snapshot.into(),
            req.quantity.into(),
            price.unit_price.into(),
            price.service_fee.into(),
            price.total_price.into(),
            OrderStatus::Reserving.to_string().into(),
            serde_json::to_value(req.shipping_address).unwrap().into(),
            req.note_to_jastiper.unwrap_or_default().into(),
            now.into(),
            now.into(),
            expired_at.into(),
        ])
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_with(&sql, values).execute(pool).await?;

    order_status_history_repo
        .insert_status_history(
            order_id,
            &OrderStatus::Reserving,
            &titipers_id.to_string(),
            &Role::System,
            Some("Pesanan berhasil dibuat"),
        )
        .await?;

    find_by_id(pool, order_id).await?.ok_or(AppError::Internal)
}

pub async fn update(
    pool: &PgPool,
    order_status_history_repo: &(dyn OrderStatusHistoryRepository + Send + Sync),
    order_id: Uuid,
    new_status: &OrderStatus,
    params: UpdateOrderParams<'_>,
) -> Result<Order> {
    let now = Utc::now();

    let mut query = Query::update();
    query
        .table(OrderIden::Order)
        .value(OrderIden::Status, Expr::value(new_status.to_string()))
        .value(OrderIden::UpdatedAt, now)
        .and_where(Expr::col(OrderIden::OrderId).eq(order_id));

    if *new_status == OrderStatus::Completed {
        query.value(OrderIden::CompletedAt, now);
    }
    if let Some(tn) = params.tracking_number {
        query.value(OrderIden::TrackingNumber, tn);
    }
    if let Some(c) = params.courier {
        query.value(OrderIden::Courier, c);
    }
    if let Some(cr) = params.cancellation_reason {
        query.value(OrderIden::CancellationReason, cr);
    }

    let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
    sqlx::query_with(&sql, values).execute(pool).await?;

    order_status_history_repo
        .insert_status_history(
            order_id,
            new_status,
            params.changed_by,
            params.actor_role,
            params.notes,
        )
        .await?;

    find_by_id(pool, order_id).await?.ok_or(AppError::Internal)
}

pub async fn delete(pool: &PgPool, order_id: Uuid) -> Result<()> {
    let (sql, values) = Query::delete()
        .from_table(OrderIden::Order)
        .and_where(Expr::col(OrderIden::OrderId).eq(order_id))
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_with(&sql, values).execute(pool).await?;
    Ok(())
}

pub async fn find_expired_pending_orders(pool: &PgPool) -> Result<Vec<Order>> {
    let now = Utc::now();

    let (sql, values) = Query::select()
        .columns([
            OrderIden::OrderId,
            OrderIden::TitipersId,
            OrderIden::JastiperId,
            OrderIden::ProductId,
            OrderIden::ProductSnapshot,
            OrderIden::Quantity,
            OrderIden::UnitPrice,
            OrderIden::ServiceFee,
            OrderIden::TotalPrice,
            OrderIden::Status,
            OrderIden::ShippingAddress,
            OrderIden::NoteToJastiper,
            OrderIden::TrackingNumber,
            OrderIden::Courier,
            OrderIden::CancellationReason,
            OrderIden::CancelledBy,
            OrderIden::CompletedAt,
            OrderIden::CreatedAt,
            OrderIden::UpdatedAt,
            OrderIden::ExpiredAt,
        ])
        .from(OrderIden::Order)
        .and_where(
            Expr::col(OrderIden::Status)
                .cast_as(Alias::new("TEXT"))
                .eq(OrderStatus::Pending.to_string()),
        )
        .and_where(Expr::col(OrderIden::ExpiredAt).lt(now))
        .build_sqlx(PostgresQueryBuilder);

    let orders = sqlx::query_as_with::<_, Order, _>(&sql, values)
        .fetch_all(pool)
        .await?;

    Ok(orders)
}
