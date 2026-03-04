use chrono::{DateTime, Utc};
use sea_query::Iden;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::types::Uuid;
use utoipa::ToSchema;
use validator::Validate;

// ── SeaQuery Identifiers ──────────────────────────────────────────
#[derive(Iden)]
pub enum OrderIden {
    Orders,
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

// ── Enums ────────────────────────────────────────────────────────
#[derive(Debug,Serialize,Deserialize,sqlx::Type,Clone,PartialEq,ToSchema)]
#[sqlx(type_name="order_status",rename_all="SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    Pending, Paid, Purchased, Shipped, Completed, Cancelled,
}

impl OrderStatus {
    pub fn valid_next(&self) -> Vec<OrderStatus> {
        match self {
            OrderStatus::Pending   => vec![OrderStatus::Paid, OrderStatus::Cancelled],
            OrderStatus::Paid      => vec![OrderStatus::Purchased, OrderStatus::Cancelled],
            OrderStatus::Purchased => vec![OrderStatus::Shipped, OrderStatus::Cancelled],
            OrderStatus::Shipped   => vec![OrderStatus::Completed, OrderStatus::Cancelled],
            OrderStatus::Completed => vec![], // terminal
            OrderStatus::Cancelled => vec![], // terminal
        }
    }
    pub fn can_transition_to(&self, next: &OrderStatus) -> bool {
        self.valid_next().contains(next)
    }
}

#[derive(Debug,Serialize,Deserialize,Clone,ToSchema)]
pub struct ProductSnapshot {
    pub product_id:      Uuid,
    pub name:            String,
    pub description:     String,
    pub image_url:       String,
    pub origin_country:  String,
    pub purchase_date:   String,
    pub unit_price:      i64,
    pub service_fee:     i64,
}

#[derive(Debug,Serialize,Deserialize,Clone,ToSchema,Validate)]
pub struct ShippingAddress {
    pub recipient_name: String,
    pub phone_number:   String,
    pub street:         String,
    pub kelurahan:      String,
    pub kecamatan:      String,
    pub city:           String,
    pub province:       String,
    pub postal_code:    String,
    pub notes:          Option<String>,
}

#[derive(Debug,Serialize,Deserialize,sqlx::FromRow,ToSchema)]
pub struct Order {
    pub order_id:           Uuid,
    pub titipers_id:        Uuid,
    pub jastiper_id:        Uuid,
    pub product_id:         Uuid,
    pub product_snapshot:   JsonValue,
    pub quantity:           i32,
    pub unit_price:         i64,
    pub service_fee:        i64,
    pub total_price:        i64,
    pub status:             OrderStatus,
    pub shipping_address:   JsonValue,
    pub note_to_jastiper:   Option<String>,
    pub tracking_number:    Option<String>,
    pub courier:            Option<String>,
    pub cancellation_reason:Option<String>,
    pub cancelled_by:       Option<String>,
    pub completed_at:       Option<DateTime<Utc>>,
    pub created_at:         DateTime<Utc>,
    pub updated_at:         DateTime<Utc>,
}

#[derive(Debug,Deserialize,ToSchema,Validate)]
pub struct CheckoutRequest {
    pub product_id:       Uuid,
    #[validate(range(min=1))]
    pub quantity:         i32,
    pub shipping_address: ShippingAddress,
    #[validate(length(max=500))]
    pub note_to_jastiper: Option<String>,
}

#[derive(Debug,Deserialize,ToSchema)]
pub struct UpdateStatusRequest {
    pub status:           OrderStatus,
    pub notes:            Option<String>,
    pub tracking_number:  Option<String>,
    pub courier:          Option<String>,
}

#[derive(Debug,Deserialize,ToSchema)]
pub struct CancelRequest {
    pub cancellation_reason: CancellationReason,
    pub notes: Option<String>,
}

#[derive(Debug,Serialize,Deserialize,sqlx::Type,Clone,ToSchema)]
#[sqlx(type_name="cancellation_reason",rename_all="SCREAMING_SNAKE_CASE")]
pub enum CancellationReason {
    OutOfStockPhysical,
    TripCancelled,
    ItemUnavailable,
    Other,
}

#[derive(Debug,Deserialize,ToSchema,Default)]
pub struct OrderFilter {
    pub status:      Option<OrderStatus>,
    pub jastiper_id: Option<Uuid>,
    pub titipers_id: Option<Uuid>,
    pub product_id:  Option<Uuid>,
    pub date_from:   Option<String>,
    pub date_to:     Option<String>,
}

#[derive(Debug,Deserialize,ToSchema,Default)]
pub struct PaginationParams {
    pub page:    Option<i64>,
    pub limit:   Option<i64>,
    pub sort_by: Option<String>,
    pub order:   Option<String>,
}

