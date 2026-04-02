use std::fmt;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use sea_query::Iden;
use uuid::Uuid;


#[derive(Iden)]
pub enum OrderStatusHistoryIden {
    #[iden = "order_status_history"]
    OrderStatusHistory,
    StatusHisId,
    OrderId,
    Status,
    ChangedBy,
    ActorRole,
    Notes,
    Timestamp,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct OrderStatusHistory {
    pub statushis_id: Uuid,
    pub order_id: Uuid,
    pub status: String,
    pub changed_by: String,
    pub actor_role: String,
    pub notes: Option<String>,
    pub timestamp: DateTime<Utc>,
}

