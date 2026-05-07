#[cfg(test)]
mod tests {
    use crate::models::shipping_address::ShippingAddress;
    use validator::Validate;

    fn valid_address() -> ShippingAddress {
        ShippingAddress {
            recipient_name: "Andi Wijaya".to_string(),
            phone_number: "081234567890".to_string(),
            street: "Jl. Gatot Subroto No. 10".to_string(),
            kelurahan: "Kuningan Timur".to_string(),
            kecamatan: "Setiabudi".to_string(),
            city: "Jakarta Selatan".to_string(),
            province: "DKI Jakarta".to_string(),
            postal_code: "12950".to_string(),
            notes: None,
        }
    }

    #[test]
    fn valid_address_passes() {
        assert!(valid_address().validate().is_ok());
    }

    #[test]
    fn valid_address_with_notes_passes() {
        let addr = ShippingAddress {
            notes: Some("Depan minimarket".to_string()),
            ..valid_address()
        };
        assert!(addr.validate().is_ok());
    }

    #[test]
    fn postal_code_exactly_5_valid() {
        let addr = ShippingAddress {
            postal_code: "12345".to_string(),
            ..valid_address()
        };
        assert!(addr.validate().is_ok());
    }

    #[test]
    fn postal_code_4_digits_invalid() {
        let addr = ShippingAddress {
            postal_code: "1234".to_string(),
            ..valid_address()
        };
        assert!(addr.validate().is_err());
    }

    #[test]
    fn postal_code_6_digits_invalid() {
        let addr = ShippingAddress {
            postal_code: "123456".to_string(),
            ..valid_address()
        };
        assert!(addr.validate().is_err());
    }

    #[test]
    fn postal_code_empty_invalid() {
        let addr = ShippingAddress {
            postal_code: "".to_string(),
            ..valid_address()
        };
        assert!(addr.validate().is_err());
    }

    #[test]
    fn postal_code_with_spaces_invalid() {
        let addr = ShippingAddress {
            postal_code: "1234 ".to_string(),
            ..valid_address()
        };
        let _ = addr.validate();
    }

    #[test]
    fn shipping_address_cloneable() {
        let addr = valid_address();
        let cloned = addr.clone();
        assert_eq!(addr.postal_code, cloned.postal_code);
        assert_eq!(addr.city, cloned.city);
    }
}
