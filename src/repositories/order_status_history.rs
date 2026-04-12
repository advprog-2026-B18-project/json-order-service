use chrono::Utc;
use sea_query::{PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::order_status_history::{OrderStatusHistory, OrderStatusHistoryIden};
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