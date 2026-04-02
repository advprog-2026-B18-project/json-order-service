use chrono::Utc;
use sea_query::{PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::order::{Order, OrderIden};
use crate::models::order_status_history::{OrderStatusHistory, OrderStatusHistoryIden};
use crate::repositories::order::find_by_id;
use crate::error::Result;
use crate::models::order_state::OrderStatus;
use crate::models::role::Role;

pub async fn insert_status_history(
    pool: &PgPool,
    order_id: Uuid,
    status: &OrderStatus,
    changed_by: &str,
    actor_role: &Role,
    notes: Option<&str>,
) -> Result<()> {
    let (sql, values) = Query::insert()
        .into_table(OrderStatusHistoryIden::OrderStatusHistory)
        .columns([
            OrderStatusHistoryIden::StatusHisId,
            OrderStatusHistoryIden::OrderId,
            OrderStatusHistoryIden::Status,
            OrderStatusHistoryIden::ChangedBy,
            OrderStatusHistoryIden::ActorRole,
            OrderStatusHistoryIden::Notes,
            OrderStatusHistoryIden::Timestamp,
        ])
        .values_panic([
            Uuid::new_v4().into(),
            order_id.into(),
            status.to_string().into(),
            changed_by.into(),
            actor_role.to_string().into(),
            notes.unwrap_or("").into(),
            Utc::now().into(),
        ])
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_with(&sql, values).execute(pool).await?;
    Ok(())
}

pub async fn update_status_history(
    pool: &PgPool,
    order_id: Uuid,
    new_status: &OrderStatus,
    changed_by: &str,
    actor_role: &Role,
    notes: Option<&str>,
    tracking_number: Option<&str>,
    courier: Option<&str>,
) -> Result<Order> {
    let now = Utc::now();

    let order = find_by_id(pool, order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    let status_str = new_status.to_string(); 

    let mut query = Query::update();
    query
        .table(OrderIden::Order)
        .value(
            OrderIden::Status,
            sea_query::Expr::cust(format!("'{}'::order_status", status_str)),
        )
        .value(OrderIden::UpdatedAt, now)
        .and_where(sea_query::Expr::col(OrderIden::OrderId).eq(order_id));

    if *new_status == OrderStatus::Completed {
        query.value(OrderIden::CompletedAt, now);
    }

    if let Some(tn) = tracking_number {
        query.value(OrderIden::TrackingNumber, tn);
    }
    if let Some(c) = courier {
        query.value(OrderIden::Courier, c);
    }

    let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
    sqlx::query_with(&sql, values).execute(pool).await?;

    insert_status_history(pool, order_id, &new_status, changed_by, actor_role, notes).await?;

    find_by_id(pool, order_id).await?.ok_or(AppError::Internal)
}

pub async fn get_status_history(
    pool: &PgPool,
    order_id: Uuid,
) -> Result<Vec<OrderStatusHistory>> {
    let (sql, values) = Query::select()
        .columns([
            OrderStatusHistoryIden::StatusHisId,
            OrderStatusHistoryIden::OrderId,
            OrderStatusHistoryIden::Status,
            OrderStatusHistoryIden::ChangedBy,
            OrderStatusHistoryIden::ActorRole,
            OrderStatusHistoryIden::Notes,
            OrderStatusHistoryIden::Timestamp,
        ])
        .from(OrderStatusHistoryIden::OrderStatusHistory)
        .and_where(sea_query::Expr::col(OrderStatusHistoryIden::OrderId).eq(order_id))
        .order_by(OrderStatusHistoryIden::Timestamp, sea_query::Order::Asc)
        .build_sqlx(PostgresQueryBuilder);

    let rows = sqlx::query_as_with::<_, OrderStatusHistory, _>(&sql, values)
        .fetch_all(pool)
        .await?;

    Ok(rows)
}