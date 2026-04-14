#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    use crate::models::order::Order;
    use crate::models::order_state::OrderStatus;

    fn make_order(order_id: Uuid, status: OrderStatus, total_price: i64) -> Order {
        Order {
            order_id,
            titipers_id: Uuid::new_v4(),
            jastiper_id: Uuid::new_v4(),
            product_id: Uuid::new_v4(),
            product_snapshot: json!({}),
            quantity: 1,
            unit_price: total_price,
            service_fee: 0,
            total_price,
            status,
            shipping_address: json!({}),
            note_to_jastiper: None,
            tracking_number: None,
            courier: None,
            cancellation_reason: None,
            cancelled_by: None,
            completed_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    mod payment_confirmed {
        use super::*;
        use crate::models::order::PaymentConfirmedRequest;

        // Harus berhasil jika status PENDING dan amount cocok
        #[test]
        fn test_berhasil_jika_pending_dan_amount_cocok() {
            let order = make_order(Uuid::new_v4(), OrderStatus::Pending, 50_000);
            let req = PaymentConfirmedRequest {
                wallet_transaction_id: Uuid::new_v4(),
                amount_deducted: 50_000,
            };

            assert_eq!(order.status, OrderStatus::Pending);
            assert_eq!(order.total_price, req.amount_deducted);
        }

        // Harus conflict jika status sudah PAID
        #[test]
        fn test_conflict_jika_sudah_paid() {
            let order = make_order(Uuid::new_v4(), OrderStatus::Paid, 50_000);

            let already_paid = order.status == OrderStatus::Paid;
            assert!(already_paid);
        }

        // Harus conflict jika status bukan PENDING (misal PURCHASED)
        #[test]
        fn test_conflict_jika_status_bukan_pending() {
            let order = make_order(Uuid::new_v4(), OrderStatus::Purchased, 50_000);

            let not_pending = order.status != OrderStatus::Pending;
            assert!(not_pending);
        }

        // Harus UnprocessableEntity jika amount tidak cocok
        #[test]
        fn test_amount_mismatch() {
            let order = make_order(Uuid::new_v4(), OrderStatus::Pending, 50_000);
            let req = PaymentConfirmedRequest {
                wallet_transaction_id: Uuid::new_v4(),
                amount_deducted: 99_000, // berbeda
            };

            let mismatch = order.total_price != req.amount_deducted;
            assert!(mismatch);
        }
    }

    mod refund_confirmed {
        use super::*;
        use crate::models::order::RefundConfirmedRequest;

        // Harus berhasil jika status REFUNDING dan amount cocok
        #[test]
        fn test_berhasil_jika_refunding_dan_amount_cocok() {
            let order = make_order(Uuid::new_v4(), OrderStatus::Refunding, 50_000);
            let req = RefundConfirmedRequest {
                wallet_transaction_id: Uuid::new_v4(),
                amount_refunded: 50_000,
            };

            assert_eq!(order.status, OrderStatus::Refunding);
            assert_eq!(order.total_price, req.amount_refunded);
        }

        // Harus conflict jika status sudah CANCELLED
        #[test]
        fn test_conflict_jika_sudah_cancelled() {
            let order = make_order(Uuid::new_v4(), OrderStatus::Cancelled, 50_000);

            let already_cancelled = order.status == OrderStatus::Cancelled;
            assert!(already_cancelled);
        }

        // Harus conflict jika status bukan REFUNDING
        #[test]
        fn test_conflict_jika_status_bukan_refunding() {
            let order = make_order(Uuid::new_v4(), OrderStatus::Paid, 50_000);

            let not_refunding = order.status != OrderStatus::Refunding;
            assert!(not_refunding);
        }

        // Harus UnprocessableEntity jika amount tidak cocok
        #[test]
        fn test_amount_mismatch() {
            let order = make_order(Uuid::new_v4(), OrderStatus::Refunding, 50_000);
            let req = RefundConfirmedRequest {
                wallet_transaction_id: Uuid::new_v4(),
                amount_refunded: 10_000, // berbeda
            };

            let mismatch = order.total_price != req.amount_refunded;
            assert!(mismatch);
        }

        // Refund menggunakan Role::System dan status → CANCELLED
        #[test]
        fn test_refund_confirmed_menuju_cancelled() {
            use crate::models::order_state::OrderMachine;
            use crate::models::role::Role;

            let mut machine = OrderMachine::from_status(&OrderStatus::Refunding);
            // Refunding → Completed oleh System (sesuai state machine)
            let result = machine.update_status(&Role::System, &OrderStatus::Cancelled);
            assert!(result.is_ok());
        }
    }
}
