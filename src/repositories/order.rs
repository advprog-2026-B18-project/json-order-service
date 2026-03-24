use chrono::Utc;
use sea_query::{PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::PgPool;
use uuid::Uuid;
use crate::models::cancelled_by::CancelledBy;
use crate::models::order::{Order, OrderIden};
use crate::repositories::order_status_history::insert_status_history;
use crate::error::{AppError, Result};
use crate::models::filter_pagination::OrderFilter;
use crate::models::order_request::CreateOrderRequest;
use crate::models::order_status_history::OrderStatus;

const ORDER_SELECT: &str = r#"SELECT order_id, titipers_id, jastiper_id, product_id,
                  product_snapshot, quantity, unit_price, service_fee, total_price,
                  status, shipping_address, note_to_jastiper, tracking_number, courier,
                  cancellation_reason::TEXT AS cancellation_reason,
                  cancelled_by::TEXT AS cancelled_by,
                  completed_at, created_at, updated_at
           FROM "order""#;

pub async fn find_all(
    pool: &PgPool,
    filter: Option<OrderFilter>,
    page: Option<i64>,
    limit: Option<i64>,
) -> Result<(Vec<Order>, i64)> {
    let final_limit = limit.unwrap_or(20).min(100);
    let offset = (page.unwrap_or(1).max(1) - 1) * final_limit;

    let orders: Vec<Order>;
    let total_count: i64;

    match &filter {
        Some(f) if f.titipers_id.is_some() && f.jastiper_id.is_some() => {
            let tid = f.titipers_id.unwrap();
            let jid = f.jastiper_id.unwrap();
            orders = sqlx::query_as::<_, Order>(&format!(
                "{} WHERE titipers_id = $1 AND jastiper_id = $2 LIMIT $3 OFFSET $4",
                ORDER_SELECT
            ))
            .bind(tid)
            .bind(jid)
            .bind(final_limit)
            .bind(offset)
            .fetch_all(pool)
            .await?;
            total_count = sqlx::query_scalar::<_, i64>(
                r#"SELECT COUNT(*) FROM "order" WHERE titipers_id = $1 AND jastiper_id = $2"#,
            )
            .bind(tid)
            .bind(jid)
            .fetch_one(pool)
            .await?;
        }
        Some(f) if f.titipers_id.is_some() => {
            let tid = f.titipers_id.unwrap();
            orders = sqlx::query_as::<_, Order>(&format!(
                "{} WHERE titipers_id = $1 LIMIT $2 OFFSET $3",
                ORDER_SELECT
            ))
            .bind(tid)
            .bind(final_limit)
            .bind(offset)
            .fetch_all(pool)
            .await?;
            total_count = sqlx::query_scalar::<_, i64>(
                r#"SELECT COUNT(*) FROM "order" WHERE titipers_id = $1"#,
            )
            .bind(tid)
            .fetch_one(pool)
            .await?;
        }
        Some(f) if f.jastiper_id.is_some() => {
            let jid = f.jastiper_id.unwrap();
            orders = sqlx::query_as::<_, Order>(&format!(
                "{} WHERE jastiper_id = $1 LIMIT $2 OFFSET $3",
                ORDER_SELECT
            ))
            .bind(jid)
            .bind(final_limit)
            .bind(offset)
            .fetch_all(pool)
            .await?;
            total_count = sqlx::query_scalar::<_, i64>(
                r#"SELECT COUNT(*) FROM "order" WHERE jastiper_id = $1"#,
            )
            .bind(jid)
            .fetch_one(pool)
            .await?;
        }
        _ => {
            orders = sqlx::query_as::<_, Order>(&format!("{} LIMIT $1 OFFSET $2", ORDER_SELECT))
                .bind(final_limit)
                .bind(offset)
                .fetch_all(pool)
                .await?;
            total_count = sqlx::query_scalar::<_, i64>(r#"SELECT COUNT(*) FROM "order""#)
                .fetch_one(pool)
                .await?;
        }
    }

    Ok((orders, total_count))
}

pub async fn find_by_id(pool: &PgPool, order_id: Uuid) -> Result<Option<Order>> {
    let order = sqlx::query_as::<_, Order>(&format!("{} WHERE order_id = $1", ORDER_SELECT))
        .bind(order_id)
        .fetch_optional(pool)
        .await?;

    Ok(order)
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
            current: order.status.to_string(),
            requested: "CANCELLED".to_string(),
            valid: vec![],
        });
    }

    let cancelled_by_str = match cancelled_by {
        CancelledBy::Jastiper => "JASTIPER",
        CancelledBy::Admin => "ADMIN",
    };

    let (sql, values) = Query::update()
        .table(OrderIden::Order)
        .value(
            OrderIden::Status,
            sea_query::Expr::cust("'CANCELLED'::order_status"),
        )
        .value(OrderIden::CancellationReason, cancellation_reason)
        .value(OrderIden::CancelledBy, cancelled_by_str)
        .value(OrderIden::UpdatedAt, now)
        .and_where(sea_query::Expr::col(OrderIden::OrderId).eq(order_id))
        .build_sqlx(PostgresQueryBuilder);

    sqlx::query_with(&sql, values).execute(pool).await?;

    insert_status_history(pool, order_id, "CANCELLED", changed_by, actor_role, notes).await?;

    find_by_id(pool, order_id).await?.ok_or(AppError::Internal)
}
