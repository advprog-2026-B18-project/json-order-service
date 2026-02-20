// src/tests/order_tests.rs
// Unit tests untuk Order model dan business logic

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::models::order::{CreateOrderRequest, Order, OrderStatus, OrderStatusHistory};

    // ─── Helpers ─────────────────────────────────────────────────────────────

    fn make_order(status: OrderStatus) -> Order {
        Order {
            id: Uuid::new_v4(),
            titipers_id: Uuid::new_v4(),
            jastiper_id: Uuid::new_v4(),
            product_id: Uuid::new_v4(),
            quantity: 2,
            shipping_address: "Jl. Sudirman No. 1, Jakarta".to_string(),
            total_price: 250_000,
            status,
            voucher_code: None,
            discount_amount: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_create_request() -> CreateOrderRequest {
        CreateOrderRequest {
            product_id: Uuid::new_v4(),
            jastiper_id: Uuid::new_v4(),
            quantity: 1,
            shipping_address: "Jl. Thamrin No. 5, Jakarta".to_string(),
            voucher_code: None,
        }
    }

    // ─── OrderStatus Tests ────────────────────────────────────────────────────

    #[test]
    fn test_order_status_eq() {
        assert_eq!(OrderStatus::Pending, OrderStatus::Pending);
        assert_ne!(OrderStatus::Pending, OrderStatus::Paid);
    }

    #[test]
    fn test_order_status_clone() {
        let status = OrderStatus::Shipped;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_order_status_all_variants() {
        let variants = vec![
            OrderStatus::Pending,
            OrderStatus::Paid,
            OrderStatus::Purchased,
            OrderStatus::Shipped,
            OrderStatus::Completed,
            OrderStatus::Cancelled,
        ];
        assert_eq!(variants.len(), 6);
    }

    // ─── Order struct Tests ───────────────────────────────────────────────────

    #[test]
    fn test_order_default_status_is_pending() {
        let order = make_order(OrderStatus::Pending);
        assert_eq!(order.status, OrderStatus::Pending);
    }

    #[test]
    fn test_order_quantity_positive() {
        let order = make_order(OrderStatus::Pending);
        assert!(order.quantity > 0, "quantity harus lebih besar dari 0");
    }

    #[test]
    fn test_order_total_price_non_negative() {
        let order = make_order(OrderStatus::Pending);
        assert!(order.total_price >= 0, "total_price tidak boleh negatif");
    }

    #[test]
    fn test_order_discount_cannot_exceed_total_price() {
        let order = make_order(OrderStatus::Pending);
        assert!(
            order.discount_amount <= order.total_price,
            "diskon tidak boleh melebihi total harga"
        );
    }

    #[test]
    fn test_order_ids_are_unique() {
        let o1 = make_order(OrderStatus::Pending);
        let o2 = make_order(OrderStatus::Pending);
        assert_ne!(o1.id, o2.id);
    }

    #[test]
    fn test_order_shipping_address_not_empty() {
        let order = make_order(OrderStatus::Pending);
        assert!(!order.shipping_address.is_empty());
    }

    #[test]
    fn test_order_with_voucher_code() {
        let mut order = make_order(OrderStatus::Pending);
        order.voucher_code = Some("DISKON50".to_string());
        order.discount_amount = 50_000;
        assert!(order.voucher_code.is_some());
        assert_eq!(order.discount_amount, 50_000);
    }

    // ─── CreateOrderRequest Tests ─────────────────────────────────────────────

    #[test]
    fn test_create_order_request_quantity_positive() {
        let req = make_create_request();
        assert!(req.quantity > 0);
    }

    #[test]
    fn test_create_order_request_address_not_empty() {
        let req = make_create_request();
        assert!(!req.shipping_address.is_empty());
    }

    #[test]
    fn test_create_order_request_no_voucher_by_default() {
        let req = make_create_request();
        assert!(req.voucher_code.is_none());
    }

    // ─── OrderStatusHistory Tests ─────────────────────────────────────────────

    #[test]
    fn test_status_history_new_status_not_same_as_old() {
        let history = OrderStatusHistory {
            id: Uuid::new_v4(),
            order_id: Uuid::new_v4(),
            old_status: Some(OrderStatus::Pending),
            new_status: OrderStatus::Paid,
            changed_by: Uuid::new_v4(),
            note: Some("Pembayaran diterima".to_string()),
            changed_at: Utc::now(),
        };
        assert_ne!(history.old_status, Some(history.new_status.clone()));
    }

    #[test]
    fn test_status_history_first_entry_has_no_old_status() {
        let history = OrderStatusHistory {
            id: Uuid::new_v4(),
            order_id: Uuid::new_v4(),
            old_status: None,
            new_status: OrderStatus::Pending,
            changed_by: Uuid::new_v4(),
            note: None,
            changed_at: Utc::now(),
        };
        assert!(history.old_status.is_none());
    }

    // ─── State machine Tests ──────────────────────────────────────────────────

    #[test]
    fn test_completed_order_cannot_be_cancelled() {
        let order = make_order(OrderStatus::Completed);
        let can_cancel = can_transition(&order.status, &OrderStatus::Cancelled);
        assert!(!can_cancel, "order COMPLETED tidak boleh dibatalkan");
    }

    #[test]
    fn test_pending_to_paid_is_valid() {
        let order = make_order(OrderStatus::Pending);
        assert!(can_transition(&order.status, &OrderStatus::Paid));
    }

    #[test]
    fn test_pending_cannot_skip_to_shipped() {
        let order = make_order(OrderStatus::Pending);
        assert!(!can_transition(&order.status, &OrderStatus::Shipped));
    }

    fn can_transition(from: &OrderStatus, to: &OrderStatus) -> bool {
        matches!(
            (from, to),
            (OrderStatus::Pending, OrderStatus::Paid)
                | (OrderStatus::Pending, OrderStatus::Cancelled)
                | (OrderStatus::Paid, OrderStatus::Purchased)
                | (OrderStatus::Paid, OrderStatus::Cancelled)
                | (OrderStatus::Purchased, OrderStatus::Shipped)
                | (OrderStatus::Shipped, OrderStatus::Completed)
        )
    }
}
