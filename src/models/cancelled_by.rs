use std::fmt;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ═══════════════════════════════════════════════════════════════════════════════
// ENUMS
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq, ToSchema)]
#[sqlx(type_name = "cancelled_by_enum", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CancelledBy {
    Jastiper,
    Admin,
}

impl CancelledBy {
    pub fn as_str(&self) -> &'static str {
        match self {
            CancelledBy::Jastiper => "JASTIPER",
            CancelledBy::Admin => "ADMIN",
        }
    }
}

/// Gunakan std::str::FromStr sebagai pengganti from_str manual,
/// sehingga konsisten dengan konvensi Rust dan bisa dipakai dengan .parse().
impl std::str::FromStr for CancelledBy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "JASTIPER" => Ok(CancelledBy::Jastiper),
            "ADMIN" => Ok(CancelledBy::Admin),
            _ => Err(format!("Nilai CancelledBy tidak valid: '{}'", s)),
        }
    }
}

impl fmt::Display for CancelledBy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}