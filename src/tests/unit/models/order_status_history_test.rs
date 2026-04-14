#[cfg(test)]
mod tests {
    use crate::models::order_state::OrderStatus;
    use crate::models::order_status_history::OrderStatusHistory;
    use crate::models::role::Role;
    use chrono::Utc;
    use uuid::Uuid;

    fn sample_history() -> OrderStatusHistory {
        OrderStatusHistory {
            status_his_id: Uuid::new_v4(),
            order_id: Uuid::new_v4(),
            status: OrderStatus::Paid,
            changed_by: "user-service".to_string(),
            actor_role: Role::System,
            notes: Some("Pembayaran dikonfirmasi".to_string()),
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn history_struct_constructable() {
        let h = sample_history();
        assert_eq!(h.status, OrderStatus::Paid);
        assert_eq!(h.actor_role, Role::System);
    }

    #[test]
    fn history_without_notes_constructable() {
        let h = OrderStatusHistory {
            notes: None,
            ..sample_history()
        };
        assert!(h.notes.is_none());
    }

    #[test]
    fn history_serializes_to_json() {
        let h = sample_history();
        let json = serde_json::to_string(&h).unwrap();

        assert!(json.contains("order_id"));
        assert!(json.contains("PAID"));
        assert!(json.contains("SYSTEM"));
    }

    #[test]
    fn history_status_serializes_as_screaming_snake() {
        let h = OrderStatusHistory {
            status: OrderStatus::RefundFailed,
            ..sample_history()
        };
        let json = serde_json::to_string(&h).unwrap();
        assert!(json.contains("REFUND_FAILED"));
    }

    #[test]
    fn history_role_serializes_as_screaming_snake() {
        let h = OrderStatusHistory {
            actor_role: Role::Jastiper,
            ..sample_history()
        };
        let json = serde_json::to_string(&h).unwrap();
        assert!(json.contains("JASTIPER"));
    }
}
