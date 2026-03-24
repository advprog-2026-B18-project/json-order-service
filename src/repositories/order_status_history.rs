use chrono::Utc;
use sea_query::{PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::PgPool;
use uuid::Uuid;
use crate::error::AppError;
use crate::models::order::{Order, OrderIden};
use crate::models::order_status_history::{OrderStatus, OrderStatusHistory, OrderStatusHistoryIden};
use crate::repositories::order::find_by_id;
use crate::error::Result;

pub async fn insert_status_history(
    pool: &PgPool,
    order_id: Uuid,
    status: &str,
    changed_by: &str,
    actor_role: &str,
    notes: Option<&str>,
) -> crate::error::Result<()> {
    let (sql, values) = Query::insert()
        .into_table(OrderStatusHistoryIden::Table)
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
            status.into(),
            changed_by.into(),
            actor_role.into(),
            notes.unwrap_or("").into(),
            Utc::now().into(),
        ])
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_with(&sql, values).execute(pool).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn update_status(
    pool: &PgPool,
    order_id: Uuid,
    new_status: &OrderStatus,
    changed_by: &str,
    actor_role: &str,
    notes: Option<&str>,
    tracking_number: Option<&str>,
    courier: Option<&str>,
) -> Result<Order> {
    let now = Utc::now();

    let order = find_by_id(pool, order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    if !order.status.can_transition_to(new_status) {
        let valid: Vec<String> = order
            .status
            .valid_next()
            .iter()
            .map(|s: &OrderStatus| s.to_string())
            .collect();
        return Err(AppError::InvalidStatusTransition {
            current: order.status.to_string(),
            requested: new_status.to_string(),
            valid,
        });
    }

    let status_str = new_status.to_string(); // pakai Display trait, bukan format!("{:?}")

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

    insert_status_history(pool, order_id, &status_str, changed_by, actor_role, notes).await?;

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
        .from(OrderStatusHistoryIden::Table)
        .and_where(sea_query::Expr::col(OrderStatusHistoryIden::OrderId).eq(order_id))
        .order_by(OrderStatusHistoryIden::Timestamp, sea_query::Order::Asc)
        .build_sqlx(PostgresQueryBuilder);

    let rows = sqlx::query_as_with::<_, OrderStatusHistory, _>(&sql, values)
        .fetch_all(pool)
        .await?;

    Ok(rows)
}