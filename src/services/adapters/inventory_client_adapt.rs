use crate::error::AppError;
use crate::services::implements::inventory_client_impl::{
    confirm_order_received, fetch_product, release_stock, reserve_stock, send_product_rating,
};
use crate::services::inventory_client::InventoryClient;
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

pub struct HttpInventoryClient;

#[async_trait]
impl InventoryClient for HttpInventoryClient {
    async fn reserve_stock(
        &self,
        product_id: Uuid,
        order_id: Uuid,
        quantity: i32,
    ) -> Result<(), AppError> {
        reserve_stock(product_id, order_id, quantity).await
    }

    async fn release_stock(
        &self,
        product_id: Uuid,
        order_id: Uuid,
        quantity: i32,
    ) -> Result<(), AppError> {
        release_stock(product_id, order_id, quantity).await
    }

    async fn fetch_product(&self, product_id: Uuid) -> Result<Value, AppError> {
        fetch_product(product_id).await
    }

    async fn send_product_rating<'a>(
        &self,
        product_id: Uuid,
        order_id: Uuid,
        rating: f64,
        review: Option<&'a str>,
        product_images: Vec<&'a str>,
    ) -> Result<(), AppError> {
        send_product_rating(product_id, order_id, rating, review, product_images).await
    }

    async fn confirm_order_received(
        &self,
        product_id: Uuid,
        order_id: Uuid,
    ) -> Result<(), AppError> {
        confirm_order_received(product_id, order_id).await
    }
}
