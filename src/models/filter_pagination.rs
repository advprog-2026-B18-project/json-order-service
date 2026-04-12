use crate::models::order_state::OrderStatus;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, ToSchema, Default)]
pub struct OrderFilter {
    pub status: Option<OrderStatus>,
    pub jastiper_id: Option<Uuid>,
    pub titipers_id: Option<Uuid>,
    pub product_id: Option<Uuid>,
    pub date_from: DateTime<Utc>,
    pub date_to: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema, Default)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub sort_by: Option<String>,
    pub order: Option<SortOrder>,
}

#[derive(Debug, Deserialize, ToSchema, Default, Clone)]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}
