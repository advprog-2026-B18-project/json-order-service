use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ENUM UNTUK ROLE
#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq, ToSchema)]
#[sqlx(type_name = "order_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    Titipers,
    Jastiper,
    Admin,
    System,
}