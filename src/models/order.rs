use crate::models::order_state::OrderStatus;
use crate::models::role::Role;
pub(crate) use crate::models::shipping_address::ShippingAddress;
use chrono::{DateTime, Utc};
use sea_query::Iden;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::types::Uuid;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Iden)]
pub enum OrderIden {
    #[iden = "order"]
    Order,
    OrderId,
    TitipersId,
    JastiperId,
    ProductId,
    ProductSnapshot,
    Quantity,
    UnitPrice,
    ServiceFee,
    TotalPrice,
    Status,
    ShippingAddress,
    NoteToJastiper,
    TrackingNumber,
    Courier,
    CancellationReason,
    CancelledBy,
    CompletedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, ToSchema, Clone, Default)]
pub struct Order {
    pub order_id: Uuid,
    pub titipers_id: Uuid,
    pub jastiper_id: Uuid,
    pub product_id: Uuid,
    pub product_snapshot: JsonValue,
    pub quantity: i32,
    pub unit_price: i64,
    pub service_fee: i64,
    pub total_price: i64,
    pub status: OrderStatus,
    pub shipping_address: JsonValue,
    pub note_to_jastiper: Option<String>,
    pub tracking_number: Option<String>,
    pub courier: Option<String>,
    pub cancellation_reason: Option<String>,
    pub cancelled_by: Option<Role>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateOrderRequest {
    pub product_id: Uuid,
    #[validate(range(min = 1))]
    pub quantity: i32,
    pub shipping_address: ShippingAddress,
    #[validate(length(max = 500))]
    pub note_to_jastiper: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateStatusRequest {
    pub status: OrderStatus,
    pub notes: Option<String>,

    /// Wajib diisi saat status = Shipped
    pub tracking_number: Option<String>,
    /// Wajib diisi saat status = Shipped
    pub courier: Option<String>,

    /// Diisi saat status = Refunding (cancel)
    pub cancellation_reason: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CancelRequest {
    #[validate(length(max = 500))]
    pub cancellation_reason: String,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct ShippedRequest {
    pub tracking_number: Option<String>,
    pub courier: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct PaymentConfirmedRequest {
    pub wallet_transaction_id: Uuid,
    pub amount_deducted: i64,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RefundConfirmedRequest {
    pub success: bool,
    pub wallet_transaction_id: Uuid,
    pub amount_refunded: i64,
    pub notes: Option<String>,
}

pub struct PriceBreakdown {
    pub unit_price: i64,
    pub service_fee: i64,
    pub total_price: i64,
}

pub struct UpdateOrderParams<'a> {
    pub changed_by: &'a str,
    pub actor_role: &'a Role,
    pub notes: Option<&'a str>,
    pub tracking_number: Option<&'a str>,
    pub courier: Option<&'a str>,
    pub cancellation_reason: Option<&'a str>,
}
