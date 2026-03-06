use chrono::{DateTime, Utc};
use sea_query::Iden;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::types::Uuid;
use std::fmt;
use utoipa::ToSchema;
use validator::Validate;

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

#[derive(Iden)]
pub enum OrderStatusHistoryIden {
    #[iden = "order_status_history"]
    Table,
    StatusHisId,
    OrderId,
    Status,
    ChangedBy,
    ActorRole,
    Notes,
    Timestamp,
}

// ═══════════════════════════════════════════════════════════════════════════════
// ENUMS
// ═══════════════════════════════════════════════════════════════════════════════

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
    /// Mengembalikan daftar status yang valid sebagai transisi berikutnya.
    /// Completed dan Cancelled adalah terminal state — tidak bisa diubah.
    pub fn valid_next(&self) -> &'static [OrderStatus] {
        use OrderStatus::*;
        match self {
            Pending => &[Paid, Cancelled],
            Paid => &[Purchased, Cancelled],
            Purchased => &[Shipped, Cancelled],
            Shipped => &[Completed, Cancelled],
            Completed | Cancelled => &[],
        }
    }

    pub fn can_transition_to(&self, next: &OrderStatus) -> bool {
        self.valid_next().contains(next)
    }

    pub fn is_terminal(&self) -> bool {
        self.valid_next().is_empty()
    }
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OrderStatus::Pending => "PENDING",
            OrderStatus::Paid => "PAID",
            OrderStatus::Purchased => "PURCHASED",
            OrderStatus::Shipped => "SHIPPED",
            OrderStatus::Completed => "COMPLETED",
            OrderStatus::Cancelled => "CANCELLED",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq, ToSchema)]
#[sqlx(type_name = "cancelled_by_enum", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CancelledBy {
    Jastiper,
    Admin,
}

impl CancelledBy {
    pub fn as_str(&self) -> &'static str {
        match self {
            CancelledBy::Jastiper => "JASTIPER",
            CancelledBy::Admin => "ADMIN",
        }
    }
}

/// Gunakan std::str::FromStr sebagai pengganti from_str manual,
/// sehingga konsisten dengan konvensi Rust dan bisa dipakai dengan .parse().
impl std::str::FromStr for CancelledBy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "JASTIPER" => Ok(CancelledBy::Jastiper),
            "ADMIN" => Ok(CancelledBy::Admin),
            _ => Err(format!("Nilai CancelledBy tidak valid: '{}'", s)),
        }
    }
}

impl fmt::Display for CancelledBy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// VALUE OBJECTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Snapshot produk yang disimpan sebagai JSONB di DB pada saat order dibuat.
/// Tipe harga konsisten dengan Order (i64) untuk menghindari mismatch saat serialisasi.
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ProductSnapshot {
    pub product_id: Uuid,
    pub name: String,
    pub description: String,
    pub image_url: String,
    pub origin_country: String,
    /// Format "YYYY-MM-DD"
    pub purchase_date: String,
    /// Dalam satuan rupiah (IDR)
    pub unit_price: i64,
    /// Dalam satuan rupiah (IDR)
    pub service_fee: i64,
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
    /// Harus tepat 5 digit sesuai format kode pos Indonesia
    #[validate(length(equal = 5))]
    pub postal_code: String,
    pub notes: Option<String>,
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

/// Row history status dari tabel order_status_history.
/// Dipisah dari StatusHistory karena field `status` di DB adalah String (bukan enum),
/// sehingga tidak perlu konversi manual saat sqlx::FromRow.
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct OrderStatusHistory {
    pub statushis_id: Uuid,
    pub order_id: Uuid,
    pub status: String,
    pub changed_by: String,
    pub actor_role: String,
    pub notes: Option<String>,
    pub timestamp: DateTime<Utc>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// REQUEST / RESPONSE STRUCTS
// ═══════════════════════════════════════════════════════════════════════════════

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
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CancelRequest {
    pub cancellation_reason: String,
    #[validate(length(max = 500))]
    pub notes: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// FILTER & PAGINATION
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, ToSchema, Default)]
pub struct OrderFilter {
    pub status: Option<OrderStatus>,
    pub jastiper_id: Option<Uuid>,
    pub titipers_id: Option<Uuid>,
    pub product_id: Option<Uuid>,
    /// Format "YYYY-MM-DD"
    pub date_from: Option<String>,
    /// Format "YYYY-MM-DD"
    pub date_to: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema, Default)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub sort_by: Option<String>,
    /// "asc" atau "desc"
    pub order: Option<String>,
}