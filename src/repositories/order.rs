use chrono::Utc;
use sea_query::Expr;
use sea_query::{PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    models::order::{
        CancelledBy, CreateOrderRequest, Order, OrderFilter, OrderIden, OrderStatus,
        OrderStatusHistoryIden,
    },
};

pub async fn find_all(
    pool: &PgPool,
    filter: Option<OrderFilter>,
    page: Option<i64>,
    limit: Option<i64>,
) -> Result<(Vec<Order>, i64)> {
    let final_limit = limit.unwrap_or(20).min(100);
    let offset = (page.unwrap_or(1).max(1) - 1) * final_limit;

    let mut data_q = Query::select();
    data_q
        .from(OrderIden::Order)
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
        ])
        .limit(final_limit as u64)
        .offset(offset as u64);

    if let Some(f) = &filter {
        if let Some(tid) = f.titipers_id {
            data_q.and_where(sea_query::Expr::col(OrderIden::TitipersId).eq(tid));
        }
        if let Some(jid) = f.jastiper_id {
            data_q.and_where(sea_query::Expr::col(OrderIden::JastiperId).eq(jid));
        }
    }

    let (sql, values) = data_q.build_sqlx(PostgresQueryBuilder);
    let orders = sqlx::query_as_with::<_, Order, _>(&sql, values)
        .fetch_all(pool)
        .await?;

    let mut count_q = Query::select();
    count_q
        .from(OrderIden::Order)
        .expr(sea_query::Expr::col(OrderIden::OrderId).count());

    if let Some(f) = &filter {
        if let Some(tid) = f.titipers_id {
            count_q.and_where(sea_query::Expr::col(OrderIden::TitipersId).eq(tid));
        }
        if let Some(jid) = f.jastiper_id {
            count_q.and_where(sea_query::Expr::col(OrderIden::JastiperId).eq(jid));
        }
    }

    let (count_sql, count_values) = count_q.build_sqlx(PostgresQueryBuilder);
    let total_count: i64 = sqlx::query_scalar_with(&count_sql, count_values)
        .fetch_one(pool)
        .await?;

    Ok((orders, total_count))
}

pub async fn find_by_id(pool: &PgPool, order_id: Uuid) -> Result<Option<Order>> {
    let (sql, values) = Query::select()
        .from(OrderIden::Order)
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
        ])
        .and_where(Expr::col(OrderIden::OrderId).eq(order_id))
        .build_sqlx(PostgresQueryBuilder);

    let order = sqlx::query_as_with::<_, Order, _>(&sql, values)
        .fetch_optional(pool)
        .await?;

    Ok(order)
}

pub async fn insert_status_history(
    pool: &PgPool,
    order_id: Uuid,
    status: &str,
    changed_by: &str,
    actor_role: &str,
    notes: Option<&str>,
) -> Result<()> {
    let statushis_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"INSERT INTO order_status_history
           (statushis_id, order_id, status, changed_by, actor_role, notes, timestamp)
           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
    )
    .bind(statushis_id)
    .bind(order_id)
    .bind(status)
    .bind(changed_by)
    .bind(actor_role)
    .bind(notes.unwrap_or(""))
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn create(
    pool: &PgPool,
    titipers_id: Uuid,
    jastiper_id: Uuid,
    _order_id: Uuid,
    req: CreateOrderRequest,
    product_snapshot: serde_json::Value,
    unit_price: i64,
    service_fee: i64,
    total_price: i64,
) -> Result<Order> {
    let order_id = Uuid::new_v4();
    let now = Utc::now();

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
        ])
        .values_panic([
            order_id.into(),
            titipers_id.into(),
            jastiper_id.into(),
            req.product_id.into(),
            product_snapshot.into(),
            req.quantity.into(),
            unit_price.into(),
            service_fee.into(),
            total_price.into(),
            sea_query::Expr::cust("'PAID'::order_status"),
            serde_json::to_value(req.shipping_address).unwrap().into(),
            req.note_to_jastiper.unwrap_or_default().into(),
            now.into(),
            now.into(),
        ])
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_with(&sql, values).execute(pool).await?;

    insert_status_history(
        pool,
        order_id,
        "PAID",
        &titipers_id.to_string(),
        "TITIPERS",
        Some("Pesanan berhasil dibuat dan pembayaran diterima"),
    )
    .await?;

    find_by_id(pool, order_id).await?.ok_or(AppError::Internal)
}

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
            .map(|s| format!("{:?}", s).to_uppercase())
            .collect();
        return Err(AppError::InvalidStatusTransition {
            current: format!("{:?}", order.status).to_uppercase(),
            requested: format!("{:?}", new_status).to_uppercase(),
            valid,
        });
    }

    let status_str = format!("{:?}", new_status).to_uppercase();
    let status_cust = format!("'{}'::order_status", status_str);

    let completed_at_sql = if *new_status == OrderStatus::Completed {
        format!(", completed_at = '{}'", now.to_rfc3339())
    } else {
        String::new()
    };

    let tracking_sql = match (tracking_number, courier) {
        (Some(tn), Some(c)) => format!(
            ", tracking_number = '{}', courier = '{}'",
            tn.replace('\'', "''"),
            c.replace('\'', "''")
        ),
        (Some(tn), None) => format!(", tracking_number = '{}'", tn.replace('\'', "''")),
        _ => String::new(),
    };

    let raw_sql = format!(
        r#"UPDATE "order" SET status = {}, updated_at = $1{}{} WHERE order_id = $2"#,
        status_cust, completed_at_sql, tracking_sql
    );

    sqlx::query(&raw_sql)
        .bind(now)
        .bind(order_id)
        .execute(pool)
        .await?;

    insert_status_history(pool, order_id, &status_str, changed_by, actor_role, notes).await?;

    find_by_id(pool, order_id).await?.ok_or(AppError::Internal)
}

pub async fn cancel_order(
    pool: &PgPool,
    order_id: Uuid,
    cancellation_reason: &str,
    cancelled_by: &CancelledBy,
    changed_by: &str,
    actor_role: &str,
    notes: Option<&str>,
) -> Result<Order> {
    let now = Utc::now();

    let order = find_by_id(pool, order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    if !order.status.can_transition_to(&OrderStatus::Cancelled) {
        return Err(AppError::InvalidStatusTransition {
            current: format!("{:?}", order.status).to_uppercase(),
            requested: "CANCELLED".to_string(),
            valid: vec![],
        });
    }

    let cancelled_by_str = match cancelled_by {
        CancelledBy::Jastiper => "JASTIPER",
        CancelledBy::Admin => "ADMIN",
    };

    sqlx::query(
        r#"UPDATE "order"
       SET status = 'CANCELLED'::order_status,
           cancellation_reason = $1::cancellation_reason,
           cancelled_by = $2,
           updated_at = $3
       WHERE order_id = $4"#,
    )
    .bind(cancellation_reason)
    .bind(cancelled_by_str)
    .bind(now)
    .bind(order_id)
    .execute(pool)
    .await?;

    insert_status_history(pool, order_id, "CANCELLED", changed_by, actor_role, notes).await?;

    find_by_id(pool, order_id).await?.ok_or(AppError::Internal)
}

pub async fn get_status_history(
    pool: &PgPool,
    order_id: Uuid,
) -> Result<Vec<crate::models::order::OrderStatusHistory>> {
    let rows = sqlx::query_as::<_, crate::models::order::OrderStatusHistory>(
        r#"SELECT statushis_id, order_id, status, changed_by, actor_role, notes, timestamp
           FROM order_status_history
           WHERE order_id = $1
           ORDER BY timestamp ASC"#,
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
