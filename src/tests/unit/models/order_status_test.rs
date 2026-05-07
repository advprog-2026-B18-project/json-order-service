#[cfg(test)]
mod parse_tests {
    use crate::models::order_state::OrderStatus;
    use std::str::FromStr;

    #[test]
    fn roundtrip_all_statuses() {
        let statuses = [
            OrderStatus::Pending,
            OrderStatus::Paid,
            OrderStatus::Purchased,
            OrderStatus::Shipped,
            OrderStatus::Completed,
            OrderStatus::Refunding,
            OrderStatus::RefundFailed,
            OrderStatus::Cancelled,
        ];
        for s in &statuses {
            let parsed = OrderStatus::from_str(s.as_str()).unwrap();
            assert_eq!(&parsed, s);
        }
    }

    #[test]
    fn invalid_status_returns_err() {
        assert!(OrderStatus::from_str("UNKNOWN").is_err());
        assert!(OrderStatus::from_str("pending").is_err()); // lowercase
    }
}
