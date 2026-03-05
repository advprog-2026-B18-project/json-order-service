use chrono::Utc;
use sea_query::{Expr, PostgresQueryBuilder, Query};
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

// ─── find_by_id ──────────────────────────────────────────────────────────────
pub async fn find_by_id(pool: &PgPool, order_id: Uuid) -> Result<Order> {
    // Gunakan sqlx query karena SeaQuery sulit handle GENERATED kolom & JSONB
    let order = sqlx::query_as!(
        Order,
        r#"
        SELECT
            order_id,
            titipers_id,
            jastiper_id,
            product_id,
            product_snapshot,
            quantity,
            unit_price,
            service_fee,
            total_price,
            status       AS "status: OrderStatus",
            shipping_address,
            note_to_jastiper,
            tracking_number,
            courier,
            cancellation_reason,
            cancelled_by AS "cancelled_by: CancelledBy",
            status_history,
            completed_at,
            created_at,
            updated_at
        FROM "order"
        WHERE order_id = $1
        "#,
        order_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Order {} not found", order_id)))?;

    Ok(order)
}

// ─── find_all ─────────────────────────────────────────────────────────────────
pub async fn find_all(
    pool: &PgPool,
    filter: Option<OrderFilter>,
    params: Option<PaginationParams>,
) -> Result<(Vec<Order>, i64)> {
    let params = params.unwrap_or_default();
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = (page - 1) * limit;
    let sort_by = params.sort_by.unwrap_or_else(|| "created_at".into());
    let order_dir = params.order.unwrap_or_else(|| "desc".into());

    // Bangun WHERE clause dinamis
    let mut conditions = vec!["1=1".to_string()];
    let mut bind_vals: Vec<String> = vec![];
    let mut bind_idx = 1usize;

    if let Some(ref f) = filter {
        if let Some(ref s) = f.status {
            conditions.push(format!("status = ${}::order_status", bind_idx));
            bind_vals.push(format!("{:?}", s).to_uppercase());
            bind_idx += 1;
        }
        if let Some(jid) = f.jastiper_id {
            conditions.push(format!("jastiper_id = ${}", bind_idx));
            bind_vals.push(jid.to_string());
            bind_idx += 1;
        }
        if let Some(tid) = f.titipers_id {
            conditions.push(format!("titipers_id = ${}", bind_idx));
            bind_vals.push(tid.to_string());
            bind_idx += 1;
        }
        if let Some(pid) = f.product_id {
            conditions.push(format!("product_id = ${}", bind_idx));
            bind_vals.push(pid.to_string());
            bind_idx += 1;
        }
        if let Some(ref df) = f.date_from {
            conditions.push(format!("created_at >= ${}::timestamptz", bind_idx));
            bind_vals.push(df.clone());
            bind_idx += 1;
        }
        if let Some(ref dt) = f.date_to {
            conditions.push(format!("created_at <= ${}::timestamptz", bind_idx));
            bind_vals.push(dt.clone());
            bind_idx += 1;
        }
    }

    let where_clause = conditions.join(" AND ");

    // Query dinamis — tidak bisa pakai query! macro karena WHERE clause dinamis
    let sql = format!(
        r#"
        SELECT
            order_id, titipers_id, jastiper_id, product_id,
            product_snapshot, quantity, unit_price, service_fee,
            total_price, status, shipping_address, note_to_jastiper,
            tracking_number, courier, cancellation_reason, cancelled_by,
            status_history, completed_at, created_at, updated_at
        FROM "order"
        WHERE {where_clause}
        ORDER BY {sort_by} {order_dir}
        LIMIT {limit} OFFSET {offset}
        "#
    );

    let count_sql = format!(r#"SELECT COUNT(*) FROM "order" WHERE {where_clause}"#);

    // Build query dengan bind values
    let mut q = sqlx::query_as::<_, Order>(&sql);
    let mut q_count = sqlx::query_scalar::<_, i64>(&count_sql);

    for val in &bind_vals {
        q = q.bind(val);
        q_count = q_count.bind(val);
    }

    let orders = q.fetch_all(pool).await?;
    let total_count = q_count.fetch_one(pool).await?;

    Ok((orders, total_count))
}

// ─── create ───────────────────────────────────────────────────────────────────
// total_price TIDAK dimasukkan — GENERATED ALWAYS di DB
pub async fn create(
    pool: &PgPool,
    titipers_id: Uuid,
    snapshot: ProductSnapshot, // dari Modul Inventory
    req: CreateOrderRequest,
) -> Result<Order> {
    let snapshot_json = serde_json::to_value(&snapshot).map_err(|_| AppError::Internal)?;
    let address_json =
        serde_json::to_value(&req.shipping_address).map_err(|_| AppError::Internal)?;
    let order_id = Uuid::new_v4();

    // Entry pertama status_history: PENDING oleh SYSTEM
    let initial_history = json!([new_history_entry(
        order_id,
        &OrderStatus::Pending,
        "SYSTEM",
        "SYSTEM",
        None
    )]);

    let order = sqlx::query_as!(
        Order,
        r#"
        INSERT INTO "order" (
            order_id, titipers_id, jastiper_id, product_id,
            product_snapshot, quantity, unit_price, service_fee,
            status, shipping_address, note_to_jastiper, status_history
        ) VALUES (
            $1, $2, $3, $4,
            $5, $6, $7, $8,
            'PENDING', $9, $10, $11
        )
        RETURNING
            order_id, titipers_id, jastiper_id, product_id,
            product_snapshot, quantity, unit_price, service_fee,
            total_price,
            status       AS "status: OrderStatus",
            shipping_address, note_to_jastiper,
            tracking_number, courier, cancellation_reason,
            cancelled_by AS "cancelled_by: CancelledBy",
            status_history, completed_at, created_at, updated_at
        "#,
        order_id,
        titipers_id,
        req.jastiper_id, // dari CreateOrderRequest (tidak ada di body, dari product data)
        req.product_id,
        snapshot_json,
        req.quantity,
        snapshot.unit_price,
        snapshot.service_fee,
        address_json,
        req.note_to_jastiper,
        initial_history,
    )
    .fetch_one(pool)
    .await?;

    Ok(order)
}

// ─── set_paid ─────────────────────────────────────────────────────────────────
// Dipanggil dari internal handler ketika Wallet konfirmasi pembayaran
pub async fn set_paid(pool: &PgPool, order_id: Uuid, wallet_transaction_id: Uuid) -> Result<Order> {
    let order = find_by_id(pool, order_id).await?;

    // Idempotency: sudah PAID → return langsung
    if order.status == OrderStatus::Paid {
        return Ok(order);
    }

    if !order.status.can_transition_to(&OrderStatus::Paid) {
        return Err(AppError::InvalidStatusTransition {
            current: format!("{:?}", order.status).to_uppercase(),
            requested: "PAID".into(),
            valid: order
                .status
                .valid_next()
                .iter()
                .map(|s| format!("{:?}", s).to_uppercase())
                .collect(),
        });
    }

    let new_entry = new_history_entry(
        order_id,
        &OrderStatus::Paid,
        &wallet_transaction_id.to_string(),
        "SYSTEM",
        Some("Payment confirmed by Wallet service".into()),
    );

    sqlx::query!(
        r#"
        UPDATE "order"
        SET
            status         = 'PAID',
            status_history = status_history || $1::jsonb,
            updated_at     = NOW()
        WHERE order_id = $2
        "#,
        json!([new_entry]),
        order_id,
    )
    .execute(pool)
    .await?;

    find_by_id(pool, order_id).await
}

// ─── update_status ────────────────────────────────────────────────────────────
// PATCH /orders/{order_id}/status — hanya Jastiper (PURCHASED atau SHIPPED)
pub async fn update_status(
    pool: &PgPool,
    order_id: Uuid,
    jastiper_id: Uuid,
    req: UpdateStatusRequest,
    actor_id: Uuid,
) -> Result<Order> {
    let order = find_by_id(pool, order_id).await?;

    // Validasi akses
    if order.jastiper_id != jastiper_id {
        return Err(AppError::Forbidden("Access denied".into()));
    }

    // Validasi state machine
    if !order.status.can_transition_to(&req.status) {
        return Err(AppError::InvalidStatusTransition {
            current: format!("{:?}", order.status).to_uppercase(),
            requested: format!("{:?}", req.status).to_uppercase(),
            valid: order
                .status
                .valid_next()
                .iter()
                .map(|s| format!("{:?}", s).to_uppercase())
                .collect(),
        });
    }

    // Validasi SHIPPED: tracking_number & courier wajib
    if req.status == OrderStatus::Shipped {
        if req.tracking_number.is_none() || req.courier.is_none() {
            return Err(AppError::Validation(
                "tracking_number and courier are required when status is SHIPPED".into(),
            ));
        }
    }

    let new_status_str = format!("{:?}", req.status).to_uppercase();
    let new_entry = new_history_entry(
        order_id,
        &req.status,
        &actor_id.to_string(),
        "JASTIPER",
        req.notes.clone(),
    );

    sqlx::query!(
        r#"
        UPDATE "order"
        SET
            status         = $1::order_status,
            tracking_number = COALESCE($2, tracking_number),
            courier         = COALESCE($3, courier),
            status_history  = status_history || $4::jsonb,
            updated_at      = NOW()
        WHERE order_id = $5
        "#,
        new_status_str,
        req.tracking_number,
        req.courier,
        json!([new_entry]),
        order_id,
    )
    .execute(pool)
    .await?;

    find_by_id(pool, order_id).await
}

// ─── confirm_receipt ──────────────────────────────────────────────────────────
// PATCH /orders/{order_id}/confirm — Titipers konfirmasi penerimaan → COMPLETED
pub async fn confirm_receipt(pool: &PgPool, order_id: Uuid, titipers_id: Uuid) -> Result<Order> {
    let order = find_by_id(pool, order_id).await?;

    // Validasi akses
    if order.titipers_id != titipers_id {
        return Err(AppError::Forbidden("Access denied".into()));
    }

    // Harus SHIPPED dulu
    if order.status != OrderStatus::Shipped {
        return Err(AppError::UnprocessableEntity(format!(
            "Order is not in SHIPPED status, current: {:?}",
            order.status
        )));
    }

    let new_entry = new_history_entry(
        order_id,
        &OrderStatus::Completed,
        &titipers_id.to_string(),
        "TITIPERS",
        Some("Penerimaan dikonfirmasi oleh Titipers".into()),
    );

    sqlx::query!(
        r#"
        UPDATE "order"
        SET
            status         = 'COMPLETED',
            completed_at   = NOW(),
            status_history = status_history || $1::jsonb,
            updated_at     = NOW()
        WHERE order_id = $2
        "#,
        json!([new_entry]),
        order_id,
    )
    .execute(pool)
    .await?;

    find_by_id(pool, order_id).await
}

// ─── cancel ───────────────────────────────────────────────────────────────────
// POST /orders/{order_id}/cancel — Jastiper atau Admin
pub async fn cancel(
    pool: &PgPool,
    order_id: Uuid,
    actor_id: Uuid,
    actor_role: &str,          // "JASTIPER" atau "ADMIN"
    jastiper_id: Option<Uuid>, // None jika actor adalah ADMIN
    req: CancelRequest,
) -> Result<Order> {
    let order = find_by_id(pool, order_id).await?;

    // Validasi akses untuk Jastiper
    if actor_role == "JASTIPER" {
        match jastiper_id {
            Some(jid) if jid != order.jastiper_id => {
                return Err(AppError::Forbidden("Access denied".into()));
            }
            None => return Err(AppError::Forbidden("Access denied".into())),
            _ => {}
        }
    }

    // Tidak bisa cancel COMPLETED atau CANCELLED
    if matches!(
        order.status,
        OrderStatus::Completed | OrderStatus::Cancelled
    ) {
        return Err(AppError::UnprocessableEntity(format!(
            "Order cannot be cancelled, current status: {:?}",
            order.status
        )));
    }

    let cancelled_by_val = if actor_role == "ADMIN" {
        "ADMIN"
    } else {
        "JASTIPER"
    };

    let new_entry = new_history_entry(
        order_id,
        &OrderStatus::Cancelled,
        &actor_id.to_string(),
        actor_role,
        req.notes.clone(),
    );

    sqlx::query!(
        r#"
        UPDATE "order"
        SET
            status               = 'CANCELLED',
            cancellation_reason  = $1,
            cancelled_by         = $2::cancelled_by_enum,
            status_history       = status_history || $3::jsonb,
            updated_at           = NOW()
        WHERE order_id = $4
        "#,
        req.cancellation_reason,
        cancelled_by_val,
        json!([new_entry]),
        order_id,
    )
    .execute(pool)
    .await?;

    // TODO: Panggil Modul Wallet → refund
    // TODO: Panggil Modul Inventory → release stok

    find_by_id(pool, order_id).await
}

// ─── auto_complete (background job) ──────────────────────────────────────────
// Dipanggil oleh scheduler harian: auto-COMPLETED setelah 7 hari SHIPPED
pub async fn auto_complete_shipped_orders(pool: &PgPool) -> Result<u64> {
    let system_entry = json!([{
        "statushis_id": Uuid::new_v4().to_string(),
        "status":       "COMPLETED",
        "changed_by":   "SYSTEM",
        "actor_role":   "SYSTEM",
        "notes":        "Auto-completed after 7 days without confirmation",
        "timestamp":    Utc::now().to_rfc3339(),
    }]);

    let result = sqlx::query!(
        r#"
        UPDATE "order"
        SET
            status         = 'COMPLETED',
            completed_at   = NOW(),
            status_history = status_history || $1::jsonb,
            updated_at     = NOW()
        WHERE
            status     = 'SHIPPED'
            AND updated_at <= NOW() - INTERVAL '7 days'
        "#,
        system_entry,
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
