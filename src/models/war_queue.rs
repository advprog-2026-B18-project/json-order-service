use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct WarQueue {
    pub id: Uuid,
    pub product_id: Uuid,
    pub titipers_id: Uuid,
    pub quantity: i32,
    pub status: String,   // Waiting | Processing | Success | Failed
    pub joined_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct JoinWarRequest {
    pub product_id: Uuid,
    pub quantity: i32,
}