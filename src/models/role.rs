use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Display;
use std::str::FromStr;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq, ToSchema)]
#[sqlx(type_name = "order_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    Titipers,
    Jastiper,
    Admin,
    System,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Titipers => "TITIPERS",
            Role::Jastiper => "JASTIPER",
            Role::Admin => "ADMIN",
            Role::System => "SYSTEM",
        }
    }
}

impl FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "TITIPERS" => Ok(Role::Titipers),
            "JASTIPER" => Ok(Role::Jastiper),
            "ADMIN" => Ok(Role::Admin),
            "SYSTEM" => Ok(Role::System),
            _ => Err(format!("Nilai Role tidak valid: '{}'", s)),
        }
    }
}

impl Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
