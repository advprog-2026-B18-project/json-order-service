use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

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
    pub postal_code: String,
    pub notes: Option<String>,
}
