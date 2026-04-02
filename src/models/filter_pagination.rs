use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;
use crate::models::order_state::OrderStatus;

// ═══════════════════════════════════════════════════════════════════════════════
// FILTER & PAGINATION
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, ToSchema, Default)]
pub struct OrderFilter {
    pub status: Option<OrderStatus>,
    pub jastiper_id: Option<Uuid>,
    pub titipers_id: Option<Uuid>,
    pub product_id: Option<Uuid>,
    /// Format "YYYY-MM-DD"
    pub date_from: Option<String>,
    /// Format "YYYY-MM-DD"
    pub date_to: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema, Default)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub sort_by: Option<String>,
    /// "asc" atau "desc"
    pub order: Option<String>,
}