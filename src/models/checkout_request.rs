use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::order::CreateOrderRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutRequest {
    pub order_id: Uuid,
    pub titipers_id: Uuid,
    pub jastiper_id: Uuid,
    pub req: CreateOrderRequest,
    pub product: serde_json::Value,
    pub idempotency_key: Uuid,
}
