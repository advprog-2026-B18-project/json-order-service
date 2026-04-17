use chrono::{DateTime, Utc};
use sea_query::Iden;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Iden)]
pub enum RatingJastiperIden {
    #[iden = "rating_jastiper"]
    RatingJastiper,
    RatingJastiperId,
    OrderId,
    TitipersId,
    JastiperRating,
    JastiperReview,
    CreatedAt,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, ToSchema, Clone)]
pub struct RatingJastiper {
    pub rating_jastiper_id: Uuid,
    pub order_id: Uuid,
    pub titipers_id: Uuid,
    pub jastiper_rating: f64,
    pub jastiper_review: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateRatingJastiperRequest {
    #[validate(range(min = 1.0, max = 5.0))]
    pub jastiper_rating: f64,
    #[validate(length(max = 1000))]
    pub jastiper_review: Option<String>,
}
