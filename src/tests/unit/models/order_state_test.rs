use super::*;

#[cfg(test)]
mod tests {
    fn machine(s: OrderStatus) -> OrderMachine {
        OrderMachine::from_status(&s)
    }

    #[test]
    fn parse_all_statuses() {
        assert_eq!(OrderStatus::from_str("PENDING").unwrap(),       OrderStatus::Pending);
        assert_eq!(OrderStatus::from_str("PAID").unwrap(),          OrderStatus::Paid);
        assert_eq!(OrderStatus::from_str("PURCHASED").unwrap(),     OrderStatus::Purchased);
        assert_eq!(OrderStatus::from_str("SHIPPED").unwrap(),       OrderStatus::Shipped);
        assert_eq!(OrderStatus::from_str("COMPLETED").unwrap(),     OrderStatus::Completed);
        assert_eq!(OrderStatus::from_str("REFUNDING").unwrap(),     OrderStatus::Refunding);
        assert_eq!(OrderStatus::from_str("REFUND_FAILED").unwrap(), OrderStatus::RefundFailed);
        assert_eq!(OrderStatus::from_str("CANCELLED").unwrap(),     OrderStatus::Cancelled);
    }

    #[test]
    fn parse_invalid_status_returns_err() {
        assert!(OrderStatus::from_str("UNKNOWN").is_err());
        assert!(OrderStatus::from_str("pending").is_err());
        assert!(OrderStatus::from_str("REFUNDFAILED").is_err());
        assert!(OrderStatus::from_str("").is_err());
    }

    #[test]
    fn display_all_statuses() {
        assert_eq!(OrderStatus::Pending.to_string(),      "PENDING");
        assert_eq!(OrderStatus::Paid.to_string(),         "PAID");
        assert_eq!(OrderStatus::Purchased.to_string(),    "PURCHASED");
        assert_eq!(OrderStatus::Shipped.to_string(),      "SHIPPED");
        assert_eq!(OrderStatus::Completed.to_string(),    "COMPLETED");
        assert_eq!(OrderStatus::Refunding.to_string(),    "REFUNDING");
        assert_eq!(OrderStatus::RefundFailed.to_string(), "REFUND_FAILED");
        assert_eq!(OrderStatus::Cancelled.to_string(),    "CANCELLED");
    }

    #[test]
    fn roundtrip_all_statuses() {
        let statuses = [
            OrderStatus::Pending,    OrderStatus::Paid,     OrderStatus::Purchased,
            OrderStatus::Shipped,    OrderStatus::Completed, OrderStatus::Refunding,
            OrderStatus::RefundFailed, OrderStatus::Cancelled,
        ];
        for s in &statuses {
            let parsed = OrderStatus::from_str(s.as_str()).expect("roundtrip gagal");
            assert_eq!(&parsed, s);
        }
    }

    #[test]
    fn current_status_reflects_initial() {
        assert_eq!(machine(OrderStatus::Pending).current_status(),    OrderStatus::Pending);
        assert_eq!(machine(OrderStatus::Paid).current_status(),       OrderStatus::Paid);
        assert_eq!(machine(OrderStatus::Shipped).current_status(),    OrderStatus::Shipped);
        assert_eq!(machine(OrderStatus::Completed).current_status(),  OrderStatus::Completed);
        assert_eq!(machine(OrderStatus::Cancelled).current_status(),  OrderStatus::Cancelled);
    }

    #[test]
    fn current_status_updates_after_transition() {
        let mut m = machine(OrderStatus::Pending);
        m.update_status(&Role::System, &OrderStatus::Paid).unwrap();
        assert_eq!(m.current_status(), OrderStatus::Paid);
    }

    #[test]
    fn pending_to_paid_by_system_ok() {
        let mut m = machine(OrderStatus::Pending);
        assert_eq!(m.update_status(&Role::System, &OrderStatus::Paid).unwrap(), OrderStatus::Paid);
    }

    #[test]
    fn pending_to_paid_by_admin_forbidden() {
        let mut m = machine(OrderStatus::Pending);
        assert!(m.update_status(&Role::Admin, &OrderStatus::Paid).is_err());
    }

    #[test]
    fn pending_to_paid_by_jastiper_forbidden() {
        let mut m = machine(OrderStatus::Pending);
        assert!(m.update_status(&Role::Jastiper, &OrderStatus::Paid).is_err());
    }

    #[test]
    fn pending_to_paid_by_titipers_forbidden() {
        let mut m = machine(OrderStatus::Pending);
        assert!(m.update_status(&Role::Titipers, &OrderStatus::Paid).is_err());
    }

    #[test]
    fn pending_to_other_status_forbidden() {
        let mut m = machine(OrderStatus::Pending);
        assert!(m.update_status(&Role::System, &OrderStatus::Purchased).is_err());
        assert!(m.update_status(&Role::System, &OrderStatus::Shipped).is_err());
        assert!(m.update_status(&Role::System, &OrderStatus::Cancelled).is_err());
    }

    #[test]
    fn pending_cancel_by_jastiper_ok() {
        assert!(machine(OrderStatus::Pending).cancel(&Role::Jastiper).is_ok());
    }

    #[test]
    fn pending_cancel_by_admin_ok() {
        assert!(machine(OrderStatus::Pending).cancel(&Role::Admin).is_ok());
    }

    #[test]
    fn pending_cancel_by_titipers_forbidden() {
        assert!(machine(OrderStatus::Pending).cancel(&Role::Titipers).is_err());
    }

    #[test]
    fn pending_cancel_by_system_forbidden() {
        assert!(machine(OrderStatus::Pending).cancel(&Role::System).is_err());
    }

    // ── PAID transitions ─────────────────────────────────────────

    #[test]
    fn paid_to_purchased_by_jastiper_ok() {
        let mut m = machine(OrderStatus::Paid);
        assert_eq!(
            m.update_status(&Role::Jastiper, &OrderStatus::Purchased).unwrap(),
            OrderStatus::Purchased
        );
    }

    #[test]
    fn paid_to_purchased_by_admin_ok() {
        let mut m = machine(OrderStatus::Paid);
        assert_eq!(
            m.update_status(&Role::Admin, &OrderStatus::Purchased).unwrap(),
            OrderStatus::Purchased
        );
    }

    #[test]
    fn paid_to_purchased_by_system_forbidden() {
        let mut m = machine(OrderStatus::Paid);
        assert!(m.update_status(&Role::System, &OrderStatus::Purchased).is_err());
    }

    #[test]
    fn paid_to_purchased_by_titipers_forbidden() {
        let mut m = machine(OrderStatus::Paid);
        assert!(m.update_status(&Role::Titipers, &OrderStatus::Purchased).is_err());
    }

    #[test]
    fn paid_to_other_status_forbidden() {
        let mut m = machine(OrderStatus::Paid);
        assert!(m.update_status(&Role::Jastiper, &OrderStatus::Shipped).is_err());
        assert!(m.update_status(&Role::Admin, &OrderStatus::Completed).is_err());
    }

    #[test]
    fn paid_cancel_by_jastiper_ok() {
        assert!(machine(OrderStatus::Paid).cancel(&Role::Jastiper).is_ok());
    }

    #[test]
    fn paid_cancel_by_admin_ok() {
        assert!(machine(OrderStatus::Paid).cancel(&Role::Admin).is_ok());
    }

    #[test]
    fn paid_cancel_by_titipers_forbidden() {
        assert!(machine(OrderStatus::Paid).cancel(&Role::Titipers).is_err());
    }

    #[test]
    fn paid_cancel_by_system_forbidden() {
        assert!(machine(OrderStatus::Paid).cancel(&Role::System).is_err());
    }

    // ── PURCHASED transitions ────────────────────────────────────

    #[test]
    fn purchased_to_shipped_by_jastiper_ok() {
        let mut m = machine(OrderStatus::Purchased);
        assert_eq!(
            m.update_status(&Role::Jastiper, &OrderStatus::Shipped).unwrap(),
            OrderStatus::Shipped
        );
    }

    #[test]
    fn purchased_to_shipped_by_admin_ok() {
        let mut m = machine(OrderStatus::Purchased);
        assert_eq!(
            m.update_status(&Role::Admin, &OrderStatus::Shipped).unwrap(),
            OrderStatus::Shipped
        );
    }

    #[test]
    fn purchased_to_shipped_by_titipers_forbidden() {
        let mut m = machine(OrderStatus::Purchased);
        assert!(m.update_status(&Role::Titipers, &OrderStatus::Shipped).is_err());
    }

    #[test]
    fn purchased_to_shipped_by_system_forbidden() {
        let mut m = machine(OrderStatus::Purchased);
        assert!(m.update_status(&Role::System, &OrderStatus::Shipped).is_err());
    }

    #[test]
    fn purchased_cancel_by_jastiper_ok() {
        assert!(machine(OrderStatus::Purchased).cancel(&Role::Jastiper).is_ok());
    }

    #[test]
    fn purchased_cancel_by_admin_ok() {
        assert!(machine(OrderStatus::Purchased).cancel(&Role::Admin).is_ok());
    }

    #[test]
    fn purchased_cancel_by_titipers_forbidden() {
        assert!(machine(OrderStatus::Purchased).cancel(&Role::Titipers).is_err());
    }

    // ── SHIPPED transitions ──────────────────────────────────────

    #[test]
    fn shipped_to_completed_by_titipers_ok() {
        let mut m = machine(OrderStatus::Shipped);
        assert_eq!(
            m.update_status(&Role::Titipers, &OrderStatus::Completed).unwrap(),
            OrderStatus::Completed
        );
    }

    #[test]
    fn shipped_to_completed_by_admin_ok() {
        let mut m = machine(OrderStatus::Shipped);
        assert_eq!(
            m.update_status(&Role::Admin, &OrderStatus::Completed).unwrap(),
            OrderStatus::Completed
        );
    }

    #[test]
    fn shipped_to_completed_by_jastiper_forbidden() {
        let mut m = machine(OrderStatus::Shipped);
        assert!(m.update_status(&Role::Jastiper, &OrderStatus::Completed).is_err());
    }

    #[test]
    fn shipped_to_completed_by_system_forbidden() {
        let mut m = machine(OrderStatus::Shipped);
        assert!(m.update_status(&Role::System, &OrderStatus::Completed).is_err());
    }

    #[test]
    fn shipped_cancel_by_admin_ok() {
        assert!(machine(OrderStatus::Shipped).cancel(&Role::Admin).is_ok());
    }

    #[test]
    fn shipped_cancel_by_jastiper_forbidden() {
        assert!(machine(OrderStatus::Shipped).cancel(&Role::Jastiper).is_err());
    }

    #[test]
    fn shipped_cancel_by_titipers_forbidden() {
        assert!(machine(OrderStatus::Shipped).cancel(&Role::Titipers).is_err());
    }

    #[test]
    fn shipped_cancel_by_system_forbidden() {
        assert!(machine(OrderStatus::Shipped).cancel(&Role::System).is_err());
    }

    // ── COMPLETED (terminal) ─────────────────────────────────────

    #[test]
    fn completed_update_always_forbidden() {
        let roles = [Role::Titipers, Role::Jastiper, Role::Admin, Role::System];
        let next_statuses = [
            OrderStatus::Pending, OrderStatus::Paid, OrderStatus::Cancelled,
        ];
        for role in &roles {
            for next in &next_statuses {
                let mut m = machine(OrderStatus::Completed);
                assert!(m.update_status(role, next).is_err(),
                        "seharusnya error: COMPLETED -> {:?} oleh {:?}", next, role);
            }
        }
    }

    #[test]
    fn completed_cancel_always_forbidden() {
        let roles = [Role::Titipers, Role::Jastiper, Role::Admin, Role::System];
        for role in &roles {
            assert!(machine(OrderStatus::Completed).cancel(role).is_err(),
                    "seharusnya error: cancel COMPLETED oleh {:?}", role);
        }
    }

    // ── REFUNDING transitions ────────────────────────────────────

    #[test]
    fn refunding_to_completed_by_system_ok() {
        let mut m = machine(OrderStatus::Refunding);
        assert_eq!(
            m.update_status(&Role::System, &OrderStatus::Completed).unwrap(),
            OrderStatus::Completed
        );
    }

    #[test]
    fn refunding_to_completed_by_admin_ok() {
        let mut m = machine(OrderStatus::Refunding);
        assert_eq!(
            m.update_status(&Role::Admin, &OrderStatus::Completed).unwrap(),
            OrderStatus::Completed
        );
    }

    #[test]
    fn refunding_to_completed_by_jastiper_forbidden() {
        let mut m = machine(OrderStatus::Refunding);
        assert!(m.update_status(&Role::Jastiper, &OrderStatus::Completed).is_err());
    }

    #[test]
    fn refunding_to_other_status_forbidden() {
        let mut m = machine(OrderStatus::Refunding);
        assert!(m.update_status(&Role::System, &OrderStatus::Cancelled).is_err());
        assert!(m.update_status(&Role::Admin, &OrderStatus::Paid).is_err());
    }

    #[test]
    fn refunding_cancel_always_forbidden() {
        let roles = [Role::Titipers, Role::Jastiper, Role::Admin, Role::System];
        for role in &roles {
            assert!(machine(OrderStatus::Refunding).cancel(role).is_err());
        }
    }

    // ── REFUND_FAILED transitions ────────────────────────────────

    #[test]
    fn refund_failed_to_completed_by_admin_ok() {
        let mut m = machine(OrderStatus::RefundFailed);
        assert_eq!(
            m.update_status(&Role::Admin, &OrderStatus::Completed).unwrap(),
            OrderStatus::Completed
        );
    }

    #[test]
    fn refund_failed_to_completed_by_system_forbidden() {
        let mut m = machine(OrderStatus::RefundFailed);
        assert!(m.update_status(&Role::System, &OrderStatus::Completed).is_err());
    }

    #[test]
    fn refund_failed_to_completed_by_jastiper_forbidden() {
        let mut m = machine(OrderStatus::RefundFailed);
        assert!(m.update_status(&Role::Jastiper, &OrderStatus::Completed).is_err());
    }

    #[test]
    fn refund_failed_cancel_always_forbidden() {
        let roles = [Role::Titipers, Role::Jastiper, Role::Admin, Role::System];
        for role in &roles {
            assert!(machine(OrderStatus::RefundFailed).cancel(role).is_err());
        }
    }

    // ── CANCELLED ──────────────────────────────────────────────────

    #[test]
    fn cancelled_update_always_forbidden() {
        let roles = [Role::Titipers, Role::Jastiper, Role::Admin, Role::System];
        for role in &roles {
            let mut m = machine(OrderStatus::Cancelled);
            assert!(m.update_status(role, &OrderStatus::Pending).is_err());
        }
    }

    #[test]
    fn cancelled_cancel_always_forbidden() {
        let roles = [Role::Titipers, Role::Jastiper, Role::Admin, Role::System];
        for role in &roles {
            assert!(machine(OrderStatus::Cancelled).cancel(role).is_err());
        }
    }

    // ── Happy-path flow ─────────────────────────────────────

    #[test]
    fn full_happy_path_pending_to_completed() {
        let mut m = machine(OrderStatus::Pending);

        m.update_status(&Role::System,   &OrderStatus::Paid).unwrap();
        assert_eq!(m.current_status(), OrderStatus::Paid);

        m.update_status(&Role::Jastiper, &OrderStatus::Purchased).unwrap();
        assert_eq!(m.current_status(), OrderStatus::Purchased);

        m.update_status(&Role::Jastiper, &OrderStatus::Shipped).unwrap();
        assert_eq!(m.current_status(), OrderStatus::Shipped);

        m.update_status(&Role::Titipers, &OrderStatus::Completed).unwrap();
        assert_eq!(m.current_status(), OrderStatus::Completed);
    }
}