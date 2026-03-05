use chrono::{DateTime, Utc};
use sea_query::Iden;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Iden)]
pub enum RatingIden {
    #[iden = "rating"]
    Rating,
    RatingId,
    OrderId,
    TitipersId,
    JastiperRating,
    JastiperReview,
    ProductRating,
    ProductReview,
    ProductImages,
    CreatedAt,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Rating {
    pub rating_id: Uuid,
    pub order_id: Uuid,
    pub titipers_id: Uuid,
    pub jastiper_rating: f64,
    pub jastiper_review: Option<String>,
    pub product_rating: f64,
    pub product_review: Option<String>,
    pub product_images: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateRatingRequest {
    #[validate(range(min = 1.0, max = 5.0))]
    pub jastiper_rating: f64,
    #[validate(length(max = 1000))]
    pub jastiper_review: Option<String>,
    #[validate(range(min = 1.0, max = 5.0))]
    pub product_rating: f64,
    #[validate(length(max = 1000))]
    pub product_review: Option<String>,
    #[validate(length(max = 3))]
    pub product_images: Option<Vec<String>>,
}
