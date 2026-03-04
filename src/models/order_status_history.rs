use chrono::{DateTime, Utc};
use sea_query::Iden;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use utoipa::ToSchema;

#[derive(Iden)]
pub enum StatusHistoryIden {
    OrderStatusHistory,
    StatusHisId,
    OrderId,
    Status,
    ChangedBy,
    ActorRole,
    Notes,
    Timestamp,
}

#[derive(Debug,Serialize,Deserialize,sqlx::FromRow,ToSchema,Clone)]
pub struct StatusHistory {
    pub statushis_id: Uuid,
    pub order_id:     Uuid,
    pub status:       String,
    pub changed_by:   String,
    pub actor_role:   String,
    pub notes:        Option<String>,
    pub timestamp:    DateTime<Utc>,
}
