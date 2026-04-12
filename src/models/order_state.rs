use std::fmt;
use std::fmt::Display;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::error::AppError;
use crate::models::role::Role;

// ENUM UNTUK STATE
#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone, PartialEq, ToSchema)]
#[sqlx(type_name = "order_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    Pending,
    Paid,
    Purchased,
    Shipped,
    Completed,
    Refunding,
    RefundFailed,
    Cancelled,
}

impl OrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderStatus::Pending => "PENDING",
            OrderStatus::Paid => "PAID",
            OrderStatus::Purchased => "PURCHASED",
            OrderStatus::Shipped => "SHIPPED",
            OrderStatus::Completed => "COMPLETED",
            OrderStatus::Refunding => "REFUNDING",
            OrderStatus::RefundFailed => "REFUND_FAILED",
            OrderStatus::Cancelled => "CANCELLED",
        }
    }
}

impl FromStr for OrderStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PENDING" => Ok(OrderStatus::Pending),
            "PAID" => Ok(OrderStatus::Paid),
            "PURCHASED" => Ok(OrderStatus::Purchased),
            "SHIPPED" => Ok(OrderStatus::Shipped),
            "COMPLETED" => Ok(OrderStatus::Completed),
            "REFUNDING" => Ok(OrderStatus::Refunding),
            "REFUND_FAILED" => Ok(OrderStatus::RefundFailed),
            "CANCELLED" => Ok(OrderStatus::Cancelled),
            _ => Err(format!("Nilai Order Status tidak valid: '{}'", s)),
        }
    }
}

impl Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// TRAIT UNTUK STATE
pub trait OrderState: Send {
    fn update_status(&self, role: &Role, next: &OrderStatus) -> Result<OrderStatus, AppError>;

    fn cancel(&self, role: &Role) -> Result<(), AppError>;

    fn current_status(&self) -> OrderStatus;
}

// WRAPPER UNTUK STATE
pub struct OrderMachine {
    current_state: Box<dyn OrderState>,
}

impl OrderMachine {
    pub fn from_status(status: &OrderStatus) -> Self {
        Self {
            current_state: make_state(status),
        }
    }

    pub fn current_status(&self) -> OrderStatus {
        self.current_state.current_status()
    }

    pub fn update_status(&mut self, role: &Role, next: &OrderStatus) -> Result<OrderStatus, AppError> {
        let result = self.current_state.update_status(role, next)?;
        self.current_state = make_state(&result);
        Ok(result)
    }

    pub fn cancel(&self, role: &Role) -> Result<(), AppError> {
        self.current_state.cancel(role)
    }
}

// SELURUH STATE
pub struct PendingState;
pub struct PaidState;
pub struct PurchasedState;
pub struct ShippedState;
pub struct CompletedState;
pub struct RefundingState;
pub struct RefundFailedState;
pub struct CancelledState;

impl OrderState for PendingState {
    fn update_status(&self, role: &Role, next: &OrderStatus) -> Result<OrderStatus, AppError> {
        match (next, role) {
            (OrderStatus::Paid, Role::System) => Ok(OrderStatus::Paid),
            _ => Err(AppError::Forbidden(
                "Status PENDING hanya bisa berubah ke PAID oleh SYSTEM".to_string()
            )),
        }
    }

    fn cancel(&self, role: &Role) -> Result<(), AppError> {
        match role {
            Role::Jastiper | Role::Admin => Ok(()),
            _ => Err(AppError::Forbidden(
                "Hanya JASTIPER atau ADMIN yang bisa cancel order PENDING".to_string()
            )),
        }
    }

    fn current_status(&self) -> OrderStatus { OrderStatus::Pending }
}

impl OrderState for PaidState {
    fn update_status(&self, role: &Role, next: &OrderStatus) -> Result<OrderStatus, AppError> {
        match (next, role) {
            (OrderStatus::Purchased, Role::Jastiper) | (OrderStatus::Purchased, Role::Admin) => {
                Ok(OrderStatus::Purchased)
            }
            _ => Err(AppError::Forbidden(
                "Status PAID hanya bisa berubah ke PURCHASED oleh JASTIPER/ADMIN".to_string()
            )),
        }
    }

    fn cancel(&self, role: &Role) -> Result<(), AppError> {
        match role {
            Role::Jastiper | Role::Admin => Ok(()),
            _ => Err(AppError::Forbidden(
                "Hanya JASTIPER atau ADMIN yang bisa cancel order PAID".to_string()
            )),
        }
    }

    fn current_status(&self) -> OrderStatus { OrderStatus::Paid }
}

impl OrderState for PurchasedState {
    fn update_status(&self, role: &Role, next: &OrderStatus) -> Result<OrderStatus, AppError> {
        match (next, role) {
            (OrderStatus::Shipped, Role::Jastiper) | (OrderStatus::Shipped, Role::Admin) => {
                Ok(OrderStatus::Shipped)
            }
            _ => Err(AppError::Forbidden(
                "Status PURCHASED hanya bisa berubah ke SHIPPED oleh JASTIPER/ADMIN".to_string()
            )),
        }
    }

    fn cancel(&self, role: &Role) -> Result<(), AppError> {
        match role {
            Role::Jastiper | Role::Admin => Ok(()),
            _ => Err(AppError::Forbidden(
                "Hanya JASTIPER atau ADMIN yang bisa cancel order PURCHASED".to_string()
            )),
        }
    }

    fn current_status(&self) -> OrderStatus { OrderStatus::Purchased }
}

impl OrderState for ShippedState {
    fn update_status(&self, role: &Role, next: &OrderStatus) -> Result<OrderStatus, AppError> {
        match (next, role) {
            (OrderStatus::Completed, Role::Titipers) | (OrderStatus::Completed, Role::Admin) => {
                Ok(OrderStatus::Completed)
            }
            _ => Err(AppError::Forbidden(
                "Status SHIPPED hanya bisa berubah ke COMPLETED oleh TITIPERS/ADMIN".to_string()
            )),
        }
    }

    fn cancel(&self, role: &Role) -> Result<(), AppError> {
        match role {
            Role::Admin => Ok(()),
            _ => Err(AppError::Forbidden(
                "Hanya ADMIN yang bisa cancel order SHIPPED".to_string()
            )),
        }
    }

    fn current_status(&self) -> OrderStatus { OrderStatus::Shipped }
}

impl OrderState for CompletedState {
    fn update_status(&self, _role: &Role, _next: &OrderStatus) -> Result<OrderStatus, AppError> {
        Err(AppError::UnprocessableEntity(
            "Order sudah COMPLETED, tidak bisa diubah".to_string()
        ))
    }

    fn cancel(&self, _role: &Role) -> Result<(), AppError> {
        Err(AppError::UnprocessableEntity(
            "Order sudah COMPLETED, tidak bisa dibatalkan".to_string()
        ))
    }

    fn current_status(&self) -> OrderStatus { OrderStatus::Completed }
}

impl OrderState for RefundingState {
    fn update_status(&self, role: &Role, next: &OrderStatus) -> Result<OrderStatus, AppError> {
        match (next, role) {
            (OrderStatus::Completed, Role::System) | (OrderStatus::Completed, Role::Admin) => {
                Ok(OrderStatus::Completed)
            }
            _ => Err(AppError::UnprocessableEntity(
                "Order sedang dalam proses REFUNDING, tidak bisa diubah".to_string()
            )),
        }
    }

    fn cancel(&self, _role: &Role) -> Result<(), AppError> {
        Err(AppError::UnprocessableEntity(
            "Order sedang dalam proses REFUNDING, tidak bisa dibatalkan".to_string()
        ))
    }

    fn current_status(&self) -> OrderStatus { OrderStatus::Refunding }

}

impl OrderState for RefundFailedState {
    fn update_status(&self, role: &Role, next: &OrderStatus) -> Result<OrderStatus, AppError> {
        match (next, role) {
            (OrderStatus::Completed, Role::Admin) => {
                Ok(OrderStatus::Completed)
            }
            _ => Err(AppError::UnprocessableEntity(
                "Order dalam status REFUNDING hanya bisa diubah oleh admin".to_string()
            )),
        }
    }

    fn cancel(&self, _role: &Role) -> Result<(), AppError> {
        Err(AppError::UnprocessableEntity(
            "Order sedang dalam proses REFUND_FAILED, tidak bisa dibatalkan".to_string()
        ))
    }

    fn current_status(&self) -> OrderStatus { OrderStatus::RefundFailed }
}

impl OrderState for CancelledState {
    fn update_status(&self, _role: &Role, _next: &OrderStatus) -> Result<OrderStatus, AppError> {
        Err(AppError::UnprocessableEntity(
            "Order sudah CANCELLED, tidak bisa diubah".to_string()
        ))
    }

    fn cancel(&self, _role: &Role) -> Result<(), AppError> {
        Err(AppError::UnprocessableEntity(
            "Order sudah CANCELLED".to_string()
        ))
    }

    fn current_status(&self) -> OrderStatus { OrderStatus::Cancelled }
}

// CONSTRUCTOR UNTUK STATE MACHINE

fn make_state(status: &OrderStatus) -> Box<dyn OrderState> {
    match status {
        OrderStatus::Pending   => Box::new(PendingState),
        OrderStatus::Paid      => Box::new(PaidState),
        OrderStatus::Purchased => Box::new(PurchasedState),
        OrderStatus::Shipped   => Box::new(ShippedState),
        OrderStatus::Completed => Box::new(CompletedState),
        OrderStatus::Refunding => Box::new(RefundingState),
        OrderStatus::RefundFailed => Box::new(RefundFailedState),
        OrderStatus::Cancelled => Box::new(CancelledState),
    }
}