use chrono::Utc;
use sea_query::Iden;
use sea_query::{PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    models::order::{
        CancelRequest, CancelledBy, CreateOrderRequest, Order, OrderFilter, OrderIden, OrderStatus,
        PaginationParams, ProductSnapshot, StatusHistory, UpdateStatusRequest,
    },
};

// ─── Helper: build StatusHistory entry ───────────────────────────────────────
fn new_history_entry(
    order_id: Uuid,
    status: &OrderStatus,
    changed_by: &str,
    actor_role: &str,
    notes: Option<String>,
) -> serde_json::Value {
    json!({
        "statushis_id": Uuid::new_v4().to_string(),
        "order_id":     order_id.to_string(),
        "status":       format!("{:?}", status).to_uppercase(),
        "changed_by":   changed_by,
        "actor_role":   actor_role,
        "notes":        notes,
        "timestamp":    Utc::now().to_rfc3339(),
    })
}

pub async fn find_all(
    pool: &PgPool,
    _filter: Option<OrderFilter>, // Tambah underscore agar tidak warning "unused"
    page: Option<i64>,
    limit: Option<i64>,
) -> Result<(Vec<Order>, i64)> {
    let final_limit = limit.unwrap_or(20).min(100);
    let offset = (page.unwrap_or(1).max(1) - 1) * final_limit;

    // --- 1. Query Data ---
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
            OrderIden::StatusHistory,
            OrderIden::CompletedAt,
            OrderIden::CreatedAt,
            OrderIden::UpdatedAt,
        ])
        .limit(final_limit as u64)
        .offset(offset as u64);

    let (sql, values) = data_q.to_owned().build_sqlx(PostgresQueryBuilder);
    let orders = sqlx::query_as_with::<_, Order, _>(&sql, values)
        .fetch_all(pool)
        .await?;

    // --- 2. Query Total Count ---
    let mut count_q = Query::select();
    count_q
        .from(OrderIden::Order)
        .expr(sea_query::Expr::col(OrderIden::OrderId).count());

    let (count_sql, count_values) = count_q.to_owned().build_sqlx(PostgresQueryBuilder);

    let total_count: i64 = sqlx::query_scalar_with(&count_sql, count_values)
        .fetch_one(pool)
        .await?;

    // --- 3. Return  ---
    Ok((orders, total_count))
}
