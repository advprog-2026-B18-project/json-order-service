use std::fmt;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use sea_query::Iden;
use uuid::Uuid;
use crate::models::order_status_history::OrderStatus::*;

// ═══════════════════════════════════════════════════════════════════════════════
// IDEN — Sea-Query table/column identifiers
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Iden)]
pub enum OrderStatusHistoryIden {
    #[iden = "order_status_history"]
    Table,
    StatusHisId,
    OrderId,
    Status,
    ChangedBy,
    ActorRole,
    Notes,
    Timestamp,
}

// ═══════════════════════════════════════════════════════════════════════════════
// ENUMS
// ═══════════════════════════════════════════════════════════════════════════════


impl OrderStatus {
    /// Mengembalikan daftar status yang valid sebagai transisi berikutnya.
    /// Completed dan Cancelled adalah terminal state — tidak bisa diubah.
    pub fn valid_next(&self) -> &'static [OrderStatus] {
        match self {
            Pending => &[Paid, Cancelled],
            Paid => &[Purchased, Cancelled],
            Purchased => &[Shipped, Cancelled],
            Shipped => &[Completed, Cancelled],
            Completed | Cancelled => &[],
        }
    }

    pub fn can_transition_to(&self, next: &OrderStatus) -> bool {
        self.valid_next().contains(next)
    }

    pub fn is_terminal(&self) -> bool {
        self.valid_next().is_empty()
    }
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OrderStatus::Pending => "PENDING",
            OrderStatus::Paid => "PAID",
            OrderStatus::Purchased => "PURCHASED",
            OrderStatus::Shipped => "SHIPPED",
            OrderStatus::Completed => "COMPLETED",
            OrderStatus::Cancelled => "CANCELLED",
        };
        write!(f, "{}", s)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// DATABASE ROW STRUCTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Row history status dari tabel order_status_history.
/// Dipisah dari StatusHistory karena field `status` di DB adalah String (bukan enum),
/// sehingga tidak perlu konversi manual saat sqlx::FromRow.
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