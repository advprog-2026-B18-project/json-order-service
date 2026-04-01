use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;
use crate::error::AppError;
use crate::models::order::Order;
use crate::models::order_request::{PaymentConfirmedRequest, RefundConfirmedRequest};
use crate::models::order_status_history::OrderStatus;
use crate::repositories::order as order_repo;
use crate::repositories::order_status_history as history_repo;

pub async fn get_order_internal(
    pool: &PgPool,
    order_id: Uuid,
) -> Result<Order, AppError> {
    order_repo::find_by_id(pool, order_id).await?
        .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))
}

pub async fn payment_confirmed(
    pool: &PgPool,
    order_id: Uuid,
    req: PaymentConfirmedRequest,
) -> Result<Order, AppError> {
    let order = order_repo::find_by_id(pool, order_id).await?
        .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    if order.status == OrderStatus::Paid {
        return Err(AppError::Conflict("Payment already confirmed".to_string()));
    }

    if order.total_price != req.amount_deducted {
        return Err(AppError::UnprocessableEntity(
            format!("Amount mismatch, expected {}", order.total_price)
        ));
    }

    info!("✅ [payment_confirmed] order_id={} payment dikonfirmasi amount={}",
        order_id, req.amount_deducted);

    let result = history_repo::update_status(
        pool, order_id, &OrderStatus::Paid,
        "SYSTEM", "SYSTEM",
        Some("Pembayaran dikonfirmasi dari Modul Wallet"),
        None, None,
    ).await?;

    Ok(result)
}

pub async fn refund_confirmed(
    pool: &PgPool,
    order_id: Uuid,
    req: RefundConfirmedRequest,
) -> Result<Order, AppError> {
    let order = order_repo::find_by_id(pool, order_id).await?
        .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    if order.status == OrderStatus::Cancelled {
        return Ok(order);
    }

    info!("✅ [refund_confirmed] order_id={} refund dikonfirmasi amount={}",
          order_id, req.amount_refunded);

    Ok(order)
}