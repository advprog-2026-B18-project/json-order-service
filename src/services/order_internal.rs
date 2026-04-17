use tracing::info;
use tracing::log::warn;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::order::{
    Order, PaymentConfirmedRequest, RefundConfirmedRequest, UpdateStatusRequest,
};
use crate::models::order_status_history::OrderStatus;
use crate::models::role::Role;
use crate::repositories::order_impl::OrderRepository;
use crate::services::order::update_status;

pub async fn get_order_internal(
    order_repo: &dyn OrderRepository,
    order_id: Uuid,
) -> Result<Order, AppError> {
    order_repo
        .find_by_id(order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))
}

pub async fn payment_confirmed(
    order_repo: &dyn OrderRepository,
    order_id: Uuid,
    req: PaymentConfirmedRequest,
) -> Result<Order, AppError> {
    let order = order_repo
        .find_by_id(order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    if order.status == OrderStatus::Paid {
        return Err(AppError::Conflict("Payment already confirmed".to_string()));
    }

    if order.status != OrderStatus::Pending {
        return Err(AppError::Conflict(format!(
            "Status harus PENDING, sekarang {:?}",
            order.status
        )));
    }

    if order.total_price != req.amount_deducted {
        return Err(AppError::UnprocessableEntity(format!(
            "Amount mismatch, expected {}",
            order.total_price
        )));
    }

    info!(
        "✅ [payment_confirmed] order_id={} payment dikonfirmasi amount={}",
        order_id, req.amount_deducted
    );

    let result = update_status(
        order_repo,
        order_id,
        Uuid::nil(),
        &Role::System,
        UpdateStatusRequest {
            status: OrderStatus::Paid,
            notes: Some("Pembayaran dikonfirmasi dari Modul Wallet".to_string()),
            tracking_number: None,
            courier: None,
            cancellation_reason: None,
        },
    )
    .await?;

    Ok(result)
}

pub async fn refund_confirmed(
    order_repo: &dyn OrderRepository,
    order_id: Uuid,
    req: RefundConfirmedRequest,
) -> Result<Order, AppError> {
    let order = order_repo
        .find_by_id(order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    if order.status == OrderStatus::Cancelled {
        return Err(AppError::Conflict("Refund already confirmed".to_string()));
    }

    if order.status != OrderStatus::Refunding && order.status != OrderStatus::RefundFailed {
        return Err(AppError::Conflict(format!(
            "Status harus REFUNDING/REFUNDFAILED, sekarang {:?}",
            order.status
        )));
    }

    if req.success && order.total_price != req.amount_refunded {
        return Err(AppError::UnprocessableEntity(format!(
            "Amount mismatch, expected {}",
            order.total_price
        )));
    }

    let (target_status, notes) = if req.success {
        info!(
            "✅ [refund_confirmed] order_id={} refund SUKSES amount={}",
            order_id, req.amount_refunded
        );
        (
            OrderStatus::Cancelled,
            "Refund dikonfirmasi dari Modul Wallet".to_string(),
        )
    } else {
        warn!(
            "❌ [refund_confirmed] order_id={} refund GAGAL reason={:?}",
            order_id, req.notes
        );
        (
            OrderStatus::RefundFailed,
            req.notes
                .unwrap_or_else(|| "Refund gagal dari Modul Wallet".to_string()),
        )
    };

    let role = if order.status == OrderStatus::RefundFailed {
        &Role::Admin
    } else {
        &Role::System
    };

    let result = update_status(
        order_repo,
        order_id,
        Uuid::nil(),
        role,
        UpdateStatusRequest {
            status: target_status,
            notes: Option::from(notes),
            tracking_number: None,
            courier: None,
            cancellation_reason: None,
        },
    )
    .await?;

    Ok(result)
}
