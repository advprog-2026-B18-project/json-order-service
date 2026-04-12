use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, Validate)]
pub struct ProductSnapshot {
    pub product_id: Uuid,
    pub name: String,
    pub description: String,
    pub image_url: String,
    pub origin_country: String,
    pub purchase_date: DateTime<Utc>,
    #[validate(range(min = 0))]
    pub unit_price: i64,
    #[validate(range(min = 0))]
    pub service_fee: i64,
}
