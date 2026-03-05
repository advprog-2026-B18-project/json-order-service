use chrono::{DateTime, Utc};
use sea_query::Iden;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::types::Uuid;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Iden)]
pub enum OrderIden {
    #[iden = "order"] // nama tabel di DB
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
    StatusHistory,
    CompletedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq, ToSchema)]
#[sqlx(type_name = "order_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    Pending,
    Paid,
    Purchased,
    Shipped,
    Completed,
    Cancelled,
}

impl OrderStatus {
    pub fn valid_next(&self) -> Vec<OrderStatus> {
        match self {
            OrderStatus::Pending => vec![OrderStatus::Paid, OrderStatus::Cancelled],
            OrderStatus::Paid => vec![OrderStatus::Purchased, OrderStatus::Cancelled],
            OrderStatus::Purchased => vec![OrderStatus::Shipped, OrderStatus::Cancelled],
            OrderStatus::Shipped => vec![OrderStatus::Completed, OrderStatus::Cancelled],
            OrderStatus::Completed => vec![], // terminal — tidak bisa diubah
            OrderStatus::Cancelled => vec![], // terminal — tidak bisa diubah
        }
    }

    pub fn can_transition_to(&self, next: &OrderStatus) -> bool {
        self.valid_next().contains(next)
    }
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq, ToSchema)]
#[sqlx(type_name = "cancelled_by_enum", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CancelledBy {
    Jastiper,
    Admin,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ProductSnapshot {
    pub product_id: Uuid,
    pub name: String,
    pub description: String,
    pub image_url: String,
    pub origin_country: String,
    pub purchase_date: String, // DATE disimpan sebagai string "YYYY-MM-DD"
    pub unit_price: i32,       // INTEGER sesuai DB schema
    pub service_fee: i32,      // INTEGER sesuai DB schema
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, Validate)]
pub struct ShippingAddress {
    pub recipient_name: String,
    pub phone_number: String,
    pub street: String,
    pub kelurahan: String,
    pub kecamatan: String,
    pub city: String,
    pub province: String,
    #[validate(length(equal = 5))]
    pub postal_code: String, // 5 digit sesuai schema
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct StatusHistory {
    pub statushis_id: Uuid,
    pub order_id: Uuid,
    pub status: OrderStatus,
    pub changed_by: String, // UUID aktor atau "SYSTEM"
    pub actor_role: String, // TITIPERS | JASTIPER | ADMIN | SYSTEM
    pub notes: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Order {
    pub order_id: Uuid,
    pub titipers_id: Uuid,
    pub jastiper_id: Uuid,
    pub product_id: Uuid,
    pub product_snapshot: JsonValue, // JSONB → ProductSnapshot
    pub quantity: i32,
    pub unit_price: i32,  // INTEGER sesuai schema
    pub service_fee: i32, // INTEGER sesuai schema
    pub total_price: i32, // INTEGER, GENERATED ALWAYS di DB
    pub status: OrderStatus,
    pub shipping_address: JsonValue, // JSONB → ShippingAddress
    pub note_to_jastiper: Option<String>,
    pub tracking_number: Option<String>,     // diisi saat SHIPPED
    pub courier: Option<String>,             // diisi saat SHIPPED
    pub cancellation_reason: Option<String>, // VARCHAR sesuai schema
    pub cancelled_by: Option<CancelledBy>,   // cancelled_by_enum
    pub status_history: JsonValue,           // JSONB ARRAY → Vec<StatusHistory>
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
    pub tracking_number: Option<String>,
    pub courier: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CancelRequest {
    pub cancellation_reason: String,
    #[validate(length(max = 500))]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema, Default)]
pub struct OrderFilter {
    pub status: Option<OrderStatus>,
    pub jastiper_id: Option<Uuid>,
    pub titipers_id: Option<Uuid>,
    pub product_id: Option<Uuid>,
    pub date_from: Option<String>, // YYYY-MM-DD
    pub date_to: Option<String>,   // YYYY-MM-DD
}

#[derive(Debug, Deserialize, ToSchema, Default)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub sort_by: Option<String>,
    pub order: Option<String>, // "asc" | "desc"
}
