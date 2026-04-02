use chrono::{DateTime, Utc};
use sea_query::Iden;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::types::Uuid;
use utoipa::ToSchema;
use crate::models::order_state::OrderStatus;
use crate::models::role::Role;

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

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
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