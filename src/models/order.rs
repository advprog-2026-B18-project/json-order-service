use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq)]
#[sqlx(type_name = "order_status", rename_all = "PascalCase")]
pub enum OrderStatus {
    Pending,
    Paid,
    Purchased,
    Shipped,
    Completed,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Order {
    pub id: Uuid,
    pub titipers_id: Uuid,
    pub jastiper_id: Uuid,
    pub product_id: Uuid,
    pub quantity: i32,
    pub shipping_address: String,
    pub total_price: i64,
    pub status: OrderStatus,
    pub voucher_code: Option<String>,
    pub discount_amount: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    pub product_id: Uuid,
    pub jastiper_id: Uuid,
    pub quantity: i32,
    pub shipping_address: String,
    pub voucher_code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrderStatusHistory {
    pub id: Uuid,
    pub order_id: Uuid,
    pub old_status: Option<OrderStatus>,
    pub new_status: OrderStatus,
    pub changed_by: Uuid,
    pub note: Option<String>,
    pub changed_at: DateTime<Utc>,
}