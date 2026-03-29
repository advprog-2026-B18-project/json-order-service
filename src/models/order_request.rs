use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;
use crate::models::order_status_history::{OrderStatus};
use crate::models::rating::Rating;
use crate::models::shipping_address::ShippingAddress;

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
    /// Sunnah diisi saat status = Completed
    pub rating: Option<Rating>,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CancelRequest {
    pub cancellation_reason: String,
    #[validate(length(max = 500))]
    pub notes: Option<String>,
}