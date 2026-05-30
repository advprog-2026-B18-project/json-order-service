use crate::models::order_state::OrderStatus;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::de;
use utoipa::ToSchema;
use uuid::Uuid;

fn opt_i64_from_str<'de, D>(d: D) -> Result<Option<i64>, D::Error>
where
    D: de::Deserializer<'de>,
{
    match Option::<String>::deserialize(d)? {
        None => Ok(None),
        Some(ref s) if s.is_empty() => Ok(None),
        Some(s) => s.parse().map(Some).map_err(de::Error::custom),
    }
}

#[derive(Debug, Deserialize, ToSchema, Default)]
pub struct OrderFilter {
    pub status: Option<OrderStatus>,
    pub jastiper_id: Option<Uuid>,
    pub titipers_id: Option<Uuid>,
    pub product_id: Option<Uuid>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, ToSchema, Default)]
pub struct PaginationParams {
    #[serde(default, deserialize_with = "opt_i64_from_str")]
    pub page: Option<i64>,
    #[serde(default, deserialize_with = "opt_i64_from_str")]
    pub limit: Option<i64>,
    pub sort_by: Option<String>,
    pub order: Option<SortOrder>,
}

#[derive(Debug, Deserialize, ToSchema, Default)]
pub struct OrderQueryParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,

    #[serde(flatten)]
    pub filter: OrderFilter,
}

#[derive(Debug, Deserialize, ToSchema, Default, Clone)]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}
