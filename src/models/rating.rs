use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Rating {
    pub id: Uuid,
    pub order_id: Uuid,
    pub titipers_id: Uuid,
    pub jastiper_id: Uuid,
    pub product_id: Uuid,
    pub jastiper_rating: Option<i16>,
    pub product_rating: Option<i16>,
    pub review: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRatingRequest {
    pub order_id: Uuid,
    pub jastiper_rating: i16,
    pub product_rating: i16,
    pub review: Option<String>,
}