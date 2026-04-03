use chrono::Utc;
use sea_query::{Alias, Cond, Expr, PostgresQueryBuilder, Query};
use sea_query::Order::{Asc, Desc};
use sea_query_binder::SqlxBinder;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::order::{CreateOrderRequest, Order, OrderIden};
use crate::repositories::order_status_history::insert_status_history;
use crate::error::{AppError, Result};
use crate::models::filter_pagination::{OrderFilter, PaginationParams, SortOrder};
use crate::models::order_state::OrderStatus;
use crate::models::role::Role;

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
            OrderIden::CompletedAt,
            OrderIden::CreatedAt,
            OrderIden::UpdatedAt,
        ])
        .expr_as(
            Expr::cust(r#"cancelled_by::TEXT"#),
            Alias::new("cancelled_by"),
        )
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
                .eq(status.to_string())
        );
    }

    cond = cond.add(Expr::col(OrderIden::CreatedAt).gte(f.date_from));
    cond = cond.add(Expr::col(OrderIden::CreatedAt).lte(f.date_to));

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
            OrderIden::CompletedAt,
            OrderIden::CreatedAt,
            OrderIden::UpdatedAt,
        ])
        .expr_as(
            Expr::cust(r#"cancelled_by::TEXT"#),
            Alias::new("cancelled_by"),
        )
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
    titipers_id: Uuid,
    jastiper_id: Uuid,
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
            sea_query::Expr::cust("'PENDING'::order_status"),
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
        &OrderStatus::Pending,
        &titipers_id.to_string(),
        &Role::Titipers,
        Some("Pesanan berhasil dibuat"),
    )
        .await?;

    find_by_id(pool, order_id).await?.ok_or(AppError::Internal)
}

pub async fn update(
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

    let mut query = Query::update();
    query
        .table(OrderIden::Order)
        .value(
            OrderIden::Status,
            Expr::cust(format!("'{}'::order_status", new_status.to_string())),
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

    insert_status_history(pool, order_id, new_status, changed_by, actor_role, notes).await?;

    find_by_id(pool, order_id).await?.ok_or(AppError::Internal)
}
