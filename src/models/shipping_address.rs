use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

// ═══════════════════════════════════════════════════════════════════════════════
// VALUE OBJECTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Snapshot produk yang disimpan sebagai JSONB di DB pada saat order dibuat.
/// Tipe harga konsisten dengan Order (i64) untuk menghindari mismatch saat serialisasi.
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