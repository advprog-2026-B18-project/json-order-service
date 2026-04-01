use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

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