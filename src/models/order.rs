use chrono::{DateTime, Utc};
use sea_query::Iden;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::types::Uuid;
use utoipa::ToSchema;
use crate::models::order_status_history::OrderStatus;

// ═══════════════════════════════════════════════════════════════════════════════
// IDEN — Sea-Query table/column identifiers
// ═══════════════════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════════════════
// DATABASE ROW STRUCTS
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Order {
    pub order_id: Uuid,
    pub titipers_id: Uuid,
    pub jastiper_id: Uuid,
    pub product_id: Uuid,
    /// JSONB — deserialisasi ke ProductSnapshot bila dibutuhkan
    pub product_snapshot: JsonValue,
    pub quantity: i32,
    pub unit_price: i64,
    pub service_fee: i64,
    pub total_price: i64,
    pub status: OrderStatus,
    /// JSONB — deserialisasi ke ShippingAddress bila dibutuhkan
    pub shipping_address: JsonValue,
    pub note_to_jastiper: Option<String>,
    /// Diisi saat status SHIPPED
    pub tracking_number: Option<String>,
    /// Diisi saat status SHIPPED
    pub courier: Option<String>,
    pub cancellation_reason: Option<String>,
    /// Raw string dari DB (kolom bertipe TEXT, bukan cancelled_by_enum)
    /// Gunakan `.parse::<CancelledBy>()` bila perlu konversi ke enum
    pub cancelled_by: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}