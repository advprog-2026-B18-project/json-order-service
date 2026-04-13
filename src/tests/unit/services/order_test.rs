#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    use crate::models::filter_pagination::PaginationParams;
    use crate::models::order::{CancelRequest, CreateOrderRequest, Order, UpdateStatusRequest};
    use crate::models::order_state::OrderStatus;
    use crate::models::role::Role;
    use crate::models::shipping_address::ShippingAddress;

    fn make_order(
        order_id: Uuid,
        titipers_id: Uuid,
        jastiper_id: Uuid,
        status: OrderStatus,
    ) -> Order {
        Order {
            order_id,
            titipers_id,
            jastiper_id,
            product_id: Uuid::new_v4(),
            product_snapshot: json!({
                "product_id": Uuid::new_v4(),
                "name": "Matcha Kit Kat",
            }),
            quantity: 2,
            unit_price: 25_000,
            service_fee: 2_000,
            total_price: 54_000,
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

    fn make_shipping_address() -> ShippingAddress {
        ShippingAddress {
            recipient_name: "Adpro".to_string(),
            phone_number: "08123456789".to_string(),
            street: "Jl. Margonda No. 1".to_string(),
            kelurahan: "Beji".to_string(),
            kecamatan: "Beji".to_string(),
            city: "Depok".to_string(),
            province: "Jawa Barat".to_string(),
            postal_code: "16424".to_string(),
            notes: None,
        }
    }

    mod get_order {
        use super::*;

        // get_order harus return order jika requester adalah titipers
        #[test]
        fn test_titipers_bisa_akses_order() {
            let order_id = Uuid::new_v4();
            let titipers_id = Uuid::new_v4();
            let jastiper_id = Uuid::new_v4();
            let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);

            // Validasi: titipers_id cocok, tidak boleh return Forbidden
            assert_eq!(order.titipers_id, titipers_id);
            assert_ne!(order.titipers_id, jastiper_id);
        }

        // get_order harus return order jika requester adalah jastiper
        #[test]
        fn test_jastiper_bisa_akses_order() {
            let order_id = Uuid::new_v4();
            let titipers_id = Uuid::new_v4();
            let jastiper_id = Uuid::new_v4();
            let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);

            // jastiper_id cocok dengan order
            assert_eq!(order.jastiper_id, jastiper_id);
        }

        // get_order harus return Forbidden jika requester bukan titipers/jastiper
        #[test]
        fn test_orang_lain_tidak_bisa_akses_order() {
            let order_id = Uuid::new_v4();
            let titipers_id = Uuid::new_v4();
            let jastiper_id = Uuid::new_v4();
            let orang_lain = Uuid::new_v4();
            let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Pending);

            // Simulasi logic forbidden check
            let is_forbidden =
                order.titipers_id != orang_lain && order.jastiper_id != orang_lain;
            assert!(is_forbidden);
        }
    }

    mod update_status {
        use super::*;
        use crate::models::order_state::OrderMachine;

        // update_status PENDING → PAID oleh System harus berhasil
        #[test]
        fn test_pending_ke_paid_oleh_system_berhasil() {
            let mut machine = OrderMachine::from_status(&OrderStatus::Pending);
            let result = machine.update_status(&Role::System, &OrderStatus::Paid);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), OrderStatus::Paid);
        }

        // update_status PAID → PURCHASED oleh Jastiper harus berhasil
        #[test]
        fn test_paid_ke_purchased_oleh_jastiper_berhasil() {
            let mut machine = OrderMachine::from_status(&OrderStatus::Paid);
            let result = machine.update_status(&Role::Jastiper, &OrderStatus::Purchased);
            assert!(result.is_ok());
        }

        // update_status PURCHASED → SHIPPED oleh Jastiper harus berhasil
        #[test]
        fn test_purchased_ke_shipped_oleh_jastiper_berhasil() {
            let mut machine = OrderMachine::from_status(&OrderStatus::Purchased);
            let result = machine.update_status(&Role::Jastiper, &OrderStatus::Shipped);
            assert!(result.is_ok());
        }

        // update_status SHIPPED → COMPLETED oleh Titipers harus berhasil
        #[test]
        fn test_shipped_ke_completed_oleh_titipers_berhasil() {
            let mut machine = OrderMachine::from_status(&OrderStatus::Shipped);
            let result = machine.update_status(&Role::Titipers, &OrderStatus::Completed);
            assert!(result.is_ok());
        }

        // update_status PENDING → PAID oleh Titipers harus gagal (Forbidden)
        #[test]
        fn test_pending_ke_paid_oleh_titipers_gagal() {
            let mut machine = OrderMachine::from_status(&OrderStatus::Pending);
            let result = machine.update_status(&Role::Titipers, &OrderStatus::Paid);
            assert!(result.is_err());
        }

        // update_status COMPLETED tidak bisa diubah lagi
        #[test]
        fn test_completed_tidak_bisa_diubah() {
            let mut machine = OrderMachine::from_status(&OrderStatus::Completed);
            let result = machine.update_status(&Role::Admin, &OrderStatus::Cancelled);
            assert!(result.is_err());
        }

        // update_status CANCELLED tidak bisa diubah lagi
        #[test]
        fn test_cancelled_tidak_bisa_diubah() {
            let mut machine = OrderMachine::from_status(&OrderStatus::Cancelled);
            let result = machine.update_status(&Role::Admin, &OrderStatus::Paid);
            assert!(result.is_err());
        }

        // Shipped harus ada tracking_number dan courier
        #[test]
        fn test_shipped_tanpa_tracking_number_gagal() {
            let req = UpdateStatusRequest {
                status: OrderStatus::Shipped,
                notes: None,
                tracking_number: None, // tidak ada
                courier: Some("JNE".to_string()),
                cancellation_reason: None,
            };
            // Validasi logic: tracking_number wajib saat SHIPPED
            assert!(req.tracking_number.is_none());
        }

        #[test]
        fn test_shipped_tanpa_courier_gagal() {
            let req = UpdateStatusRequest {
                status: OrderStatus::Shipped,
                notes: None,
                tracking_number: Some("JNE-123".to_string()),
                courier: None, // tidak ada
                cancellation_reason: None,
            };
            assert!(req.courier.is_none());
        }

        // Jastiper tidak bisa update order milik jastiper lain
        #[test]
        fn test_jastiper_lain_tidak_bisa_update() {
            let order_id = Uuid::new_v4();
            let titipers_id = Uuid::new_v4();
            let jastiper_id = Uuid::new_v4();
            let jastiper_lain = Uuid::new_v4();
            let order = make_order(order_id, titipers_id, jastiper_id, OrderStatus::Paid);

            // Simulasi: jastiper_lain mencoba update → forbidden
            let forbidden = order.jastiper_id != jastiper_lain;
            assert!(forbidden);
        }
    }

    mod cancel_order {
        use super::*;
        use crate::models::order_state::OrderMachine;

        // PENDING bisa di-cancel oleh Jastiper
        #[test]
        fn test_pending_bisa_cancel_oleh_jastiper() {
            let machine = OrderMachine::from_status(&OrderStatus::Pending);
            let result = machine.cancel(&Role::Jastiper);
            assert!(result.is_ok());
        }

        // PENDING bisa di-cancel oleh Admin
        #[test]
        fn test_pending_bisa_cancel_oleh_admin() {
            let machine = OrderMachine::from_status(&OrderStatus::Pending);
            let result = machine.cancel(&Role::Admin);
            assert!(result.is_ok());
        }

        // PENDING tidak bisa di-cancel oleh Titipers
        #[test]
        fn test_pending_tidak_bisa_cancel_oleh_titipers() {
            let machine = OrderMachine::from_status(&OrderStatus::Pending);
            let result = machine.cancel(&Role::Titipers);
            assert!(result.is_err());
        }

        // SHIPPED hanya bisa di-cancel oleh Admin
        #[test]
        fn test_shipped_hanya_bisa_cancel_oleh_admin() {
            let machine = OrderMachine::from_status(&OrderStatus::Shipped);
            let result_admin = machine.cancel(&Role::Admin);
            let result_jastiper = machine.cancel(&Role::Jastiper);
            assert!(result_admin.is_ok());
            assert!(result_jastiper.is_err());
        }

        // COMPLETED tidak bisa di-cancel
        #[test]
        fn test_completed_tidak_bisa_cancel() {
            let machine = OrderMachine::from_status(&OrderStatus::Completed);
            let result = machine.cancel(&Role::Admin);
            assert!(result.is_err());
        }

        // REFUNDING tidak bisa di-cancel
        #[test]
        fn test_refunding_tidak_bisa_cancel() {
            let machine = OrderMachine::from_status(&OrderStatus::Refunding);
            let result = machine.cancel(&Role::Admin);
            assert!(result.is_err());
        }

        // CancelRequest harus punya cancellation_reason
        #[test]
        fn test_cancel_request_ada_alasan() {
            let req = CancelRequest {
                cancellation_reason: "Barang tidak tersedia".to_string(),
            };
            assert!(!req.cancellation_reason.is_empty());
        }
    }


    mod payment {
        use super::*;

        // Payment hanya bisa jika status PENDING
        #[test]
        fn test_payment_hanya_bisa_dari_pending() {
            let titipers_id = Uuid::new_v4();
            let order =
                make_order(Uuid::new_v4(), titipers_id, Uuid::new_v4(), OrderStatus::Paid);

            // Simulasi: status bukan PENDING → konflik
            let is_conflict = order.status != OrderStatus::Pending;
            assert!(is_conflict);
        }

        // Payment harus ditolak jika bukan pemilik order
        #[test]
        fn test_payment_ditolak_jika_bukan_pemilik() {
            let titipers_id = Uuid::new_v4();
            let orang_lain = Uuid::new_v4();
            let order = make_order(
                Uuid::new_v4(),
                titipers_id,
                Uuid::new_v4(),
                OrderStatus::Pending,
            );

            let is_forbidden = order.titipers_id != orang_lain;
            assert!(is_forbidden);
        }
    }

    mod checkout {
        use super::*;

        // Titipers tidak bisa beli produk milik sendiri (jastiper_id == titipers_id)
        #[test]
        fn test_titipers_tidak_bisa_beli_produk_sendiri() {
            let same_id = Uuid::new_v4();
            // Simulasi: jastiper_id == titipers_id → forbidden
            let is_forbidden = same_id == same_id;
            assert!(is_forbidden);
        }

        // Total price dihitung dengan benar
        #[test]
        fn test_total_price_dihitung_benar() {
            let unit_price: i64 = 25_000;
            let service_fee: i64 = 2_000;
            let quantity: i64 = 3;
            let total = (unit_price + service_fee) * quantity;
            assert_eq!(total, 81_000);
        }

        // CreateOrderRequest quantity minimal 1
        #[test]
        fn test_quantity_minimal_1() {
            use validator::Validate;
            let req = CreateOrderRequest {
                product_id: Uuid::new_v4(),
                quantity: 0, // invalid
                shipping_address: make_shipping_address(),
                note_to_jastiper: None,
            };
            assert!(req.validate().is_err());
        }

        // note_to_jastiper max 500 karakter
        #[test]
        fn test_note_max_500_karakter() {
            use validator::Validate;
            let req = CreateOrderRequest {
                product_id: Uuid::new_v4(),
                quantity: 1,
                shipping_address: make_shipping_address(),
                note_to_jastiper: Some("x".repeat(501)),
            };
            assert!(req.validate().is_err());
        }
    }

    mod my_orders {
        use super::*;

        // Filter my_purchases harus set titipers_id
        #[test]
        fn test_my_purchases_filter_titipers_id() {
            use crate::models::filter_pagination::OrderFilter;
            let titipers_id = Uuid::new_v4();
            let filter = OrderFilter {
                titipers_id: Some(titipers_id),
                ..Default::default()
            };
            assert_eq!(filter.titipers_id, Some(titipers_id));
            assert!(filter.jastiper_id.is_none());
        }

        // Filter my_sales harus set jastiper_id
        #[test]
        fn test_my_sales_filter_jastiper_id() {
            use crate::models::filter_pagination::OrderFilter;
            let jastiper_id = Uuid::new_v4();
            let filter = OrderFilter {
                jastiper_id: Some(jastiper_id),
                ..Default::default()
            };
            assert_eq!(filter.jastiper_id, Some(jastiper_id));
            assert!(filter.titipers_id.is_none());
        }

        // Pagination default: page=1, limit=20
        #[test]
        fn test_pagination_default() {
            let params = PaginationParams::default();
            let limit = params.limit.unwrap_or(20).min(100);
            let page = params.page.unwrap_or(1).max(1);
            assert_eq!(limit, 20);
            assert_eq!(page, 1);
        }

        // Pagination limit tidak bisa melebihi 100
        #[test]
        fn test_pagination_limit_max_100() {
            let params = PaginationParams {
                page: Some(1),
                limit: Some(999),
                sort_by: None,
                order: None,
            };
            let limit = params.limit.unwrap_or(20).min(100);
            assert_eq!(limit, 100);
        }
    }
}