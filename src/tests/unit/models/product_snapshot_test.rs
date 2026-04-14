#[cfg(test)]
mod tests {
    use crate::models::product_snapshot::ProductSnapshot;
    use chrono::Utc;
    use uuid::Uuid;
    use validator::Validate;

    fn valid_snapshot() -> ProductSnapshot {
        ProductSnapshot {
            product_id: Uuid::new_v4(),
            name: "Minyak Zaitun Extra Virgin".to_string(),
            description: "Minyak zaitun import dari Spanyol".to_string(),
            image_url: "https://example.com/img/minyak.jpg".to_string(),
            origin_country: "Spain".to_string(),
            purchase_date: Utc::now(),
            unit_price: 150_000,
            service_fee: 10_000,
        }
    }

    #[test]
    fn valid_snapshot_passes() {
        assert!(valid_snapshot().validate().is_ok());
    }

    #[test]
    fn unit_price_zero_valid() {
        let s = ProductSnapshot {
            unit_price: 0,
            ..valid_snapshot()
        };
        assert!(s.validate().is_ok());
    }

    #[test]
    fn unit_price_positive_valid() {
        let s = ProductSnapshot {
            unit_price: 1_000_000,
            ..valid_snapshot()
        };
        assert!(s.validate().is_ok());
    }

    #[test]
    fn unit_price_negative_invalid() {
        let s = ProductSnapshot {
            unit_price: -1,
            ..valid_snapshot()
        };
        assert!(s.validate().is_err());
    }

    #[test]
    fn service_fee_zero_valid() {
        let s = ProductSnapshot {
            service_fee: 0,
            ..valid_snapshot()
        };
        assert!(s.validate().is_ok());
    }

    #[test]
    fn service_fee_positive_valid() {
        let s = ProductSnapshot {
            service_fee: 5_000,
            ..valid_snapshot()
        };
        assert!(s.validate().is_ok());
    }

    #[test]
    fn service_fee_negative_invalid() {
        let s = ProductSnapshot {
            service_fee: -1,
            ..valid_snapshot()
        };
        assert!(s.validate().is_err());
    }

    #[test]
    fn both_negative_invalid() {
        let s = ProductSnapshot {
            unit_price: -100,
            service_fee: -50,
            ..valid_snapshot()
        };
        assert!(s.validate().is_err());
    }

    #[test]
    fn snapshot_cloneable() {
        let s = valid_snapshot();
        let cloned = s.clone();
        assert_eq!(s.product_id, cloned.product_id);
        assert_eq!(s.unit_price, cloned.unit_price);
    }
}
