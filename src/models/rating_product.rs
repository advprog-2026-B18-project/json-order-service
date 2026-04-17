use chrono::{DateTime, Utc};
use sea_query::Iden;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Iden)]
pub enum RatingProductIden {
    #[iden = "rating_product"]
    RatingProduct,
    RatingProductId,
    OrderId,
    TitipersId,
    ProductRating,
    ProductReview,
    ProductImages,
    CreatedAt,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, ToSchema, Clone)]
pub struct RatingProduct {
    pub rating_product_id: Uuid,
    pub order_id: Uuid,
    pub titipers_id: Uuid,
    pub product_rating: f64,
    pub product_review: Option<String>,
    pub product_images: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateRatingProductRequest {
    #[validate(range(min = 1.0, max = 5.0))]
    pub product_rating: f64,
    #[validate(length(max = 1000))]
    pub product_review: Option<String>,
    #[validate(length(max = 3))]
    pub product_images: Option<Vec<String>>,
}
