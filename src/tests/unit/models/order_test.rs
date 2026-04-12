#[cfg(test)]
mod tests {
    use crate::models::order::{CancelRequest, CreateOrderRequest};
    use crate::models::shipping_address::ShippingAddress;
    use uuid::Uuid;
    use validator::Validate;

    fn valid_address() -> ShippingAddress {
        ShippingAddress {
            recipient_name: "Budi Santoso".to_string(),
            phone_number: "081234567890".to_string(),
            street: "Jl. Sudirman No. 1".to_string(),
            kelurahan: "Senayan".to_string(),
            kecamatan: "Kebayoran Baru".to_string(),
            city: "Jakarta Selatan".to_string(),
            province: "DKI Jakarta".to_string(),
            postal_code: "12190".to_string(),
            notes: None,
        }
    }

    fn valid_create_order() -> CreateOrderRequest {
        CreateOrderRequest {
            product_id: Uuid::new_v4(),
            quantity: 1,
            shipping_address: valid_address(),
            note_to_jastiper: None,
        }
    }

    #[test]
    fn create_order_valid() {
        assert!(valid_create_order().validate().is_ok());
    }

    #[test]
    fn create_order_quantity_zero_invalid() {
        let req = CreateOrderRequest {
            quantity: 0,
            ..valid_create_order()
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn create_order_quantity_negative_invalid() {
        let req = CreateOrderRequest {
            quantity: -5,
            ..valid_create_order()
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn create_order_quantity_one_valid() {
        let req = CreateOrderRequest {
            quantity: 1,
            ..valid_create_order()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn create_order_quantity_large_valid() {
        let req = CreateOrderRequest {
            quantity: 9999,
            ..valid_create_order()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn create_order_note_within_limit_valid() {
        let req = CreateOrderRequest {
            note_to_jastiper: Some("a".repeat(500)),
            ..valid_create_order()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn create_order_note_exceeds_limit_invalid() {
        let req = CreateOrderRequest {
            note_to_jastiper: Some("a".repeat(501)),
            ..valid_create_order()
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn create_order_note_none_valid() {
        let req = CreateOrderRequest {
            note_to_jastiper: None,
            ..valid_create_order()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn cancel_request_valid() {
        let req = CancelRequest {
            cancellation_reason: "Salah pesan produk".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn cancel_request_reason_at_limit_valid() {
        let req = CancelRequest {
            cancellation_reason: "a".repeat(500),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn cancel_request_reason_exceeds_limit_invalid() {
        let req = CancelRequest {
            cancellation_reason: "a".repeat(501),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn cancel_request_empty_reason_valid() {
        let req = CancelRequest {
            cancellation_reason: String::new(),
        };
        assert!(req.validate().is_ok());
    }
}
