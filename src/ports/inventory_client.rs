use crate::error::AppError;
use async_trait::async_trait;
use uuid::Uuid;

#[mockall::automock]
#[async_trait]
pub trait InventoryClient: Send + Sync {
    async fn reserve_stock(
        &self,
        product_id: Uuid,
        order_id: Uuid,
        quantity: i32,
    ) -> Result<(), AppError>;

    async fn release_stock(
        &self,
        product_id: Uuid,
        order_id: Uuid,
        quantity: i32,
    ) -> Result<(), AppError>;

    async fn fetch_product(&self, product_id: Uuid) -> Result<serde_json::Value, AppError>;

    async fn send_product_rating<'a>(
        &self,
        product_id: Uuid,
        order_id: Uuid,
        rating: f64,
        review: Option<&'a str>,
        product_images: Vec<&'a str>,
    ) -> Result<(), AppError>;
}
