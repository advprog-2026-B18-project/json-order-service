use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::order::{Order, PaymentConfirmedRequest, RefundConfirmedRequest, UpdateStatusRequest};
use crate::models::order_status_history::OrderStatus;
use crate::models::role::Role;
use crate::repositories::order as order_repo;
use crate::services::order::update_status;

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

    if order.status != OrderStatus::Pending {
        return Err(AppError::Conflict(
            format!("Status harus PENDING, sekarang {:?}", order.status)
        ));
    }

    if order.total_price != req.amount_deducted {
        return Err(AppError::UnprocessableEntity(
            format!("Amount mismatch, expected {}", order.total_price)
        ));
    }

    info!("✅ [payment_confirmed] order_id={} payment dikonfirmasi amount={}",
        order_id, req.amount_deducted);

    let result = update_status(
        pool, order_id, Uuid::nil(), &Role::System,
        UpdateStatusRequest {
            status: OrderStatus::Paid,
            notes: Some("Pembayaran dikonfirmasi dari Modul Wallet".to_string()),
            tracking_number: None,
            courier: None,
            cancellation_reason: None,
        }
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
        return Err(AppError::Conflict("Refund already confirmed".to_string()));
    }

    if order.status != OrderStatus::Refunding {
        return Err(AppError::Conflict(
            format!("Status harus REFUNDING, sekarang {:?}", order.status)
        ));
    }

    if order.total_price != req.amount_refunded {
        return Err(AppError::UnprocessableEntity(
            format!("Amount mismatch, expected {}", order.total_price)
        ));
    }

    info!("✅ [refund_confirmed] order_id={} refund dikonfirmasi amount={}",
          order_id, req.amount_refunded);

    let result = update_status(
        pool, order_id, Uuid::nil(), &Role::System,
        UpdateStatusRequest {
            status: OrderStatus::Cancelled,
            notes: Some("Refund dikonfirmasi dari Modul Wallet".to_string()),
            tracking_number: None,
            courier: None,
            cancellation_reason: None,
        }
    ).await?;

    Ok(result)
}