#[cfg(test)]
mod tests {
    use crate::models::rating_jastiper::CreateRatingJastiperRequest;
    use validator::Validate;

    fn valid_request() -> CreateRatingJastiperRequest {
        CreateRatingJastiperRequest {
            jastiper_rating: 4.5,
            jastiper_review: Some("Jastiper cepat dan ramah".to_string()),
        }
    }

    #[test]
    fn valid_request_passes() {
        assert!(valid_request().validate().is_ok());
    }

    #[test]
    fn valid_request_no_review_passes() {
        let req = CreateRatingJastiperRequest {
            jastiper_review: None,
            ..valid_request()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn rating_min_boundary_valid() {
        let req = CreateRatingJastiperRequest {
            jastiper_rating: 1.0,
            ..valid_request()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn rating_max_boundary_valid() {
        let req = CreateRatingJastiperRequest {
            jastiper_rating: 5.0,
            ..valid_request()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn rating_mid_valid() {
        let req = CreateRatingJastiperRequest {
            jastiper_rating: 3.0,
            ..valid_request()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn rating_below_min_invalid() {
        let req = CreateRatingJastiperRequest {
            jastiper_rating: 0.9,
            ..valid_request()
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn rating_zero_invalid() {
        let req = CreateRatingJastiperRequest {
            jastiper_rating: 0.0,
            ..valid_request()
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn rating_negative_invalid() {
        let req = CreateRatingJastiperRequest {
            jastiper_rating: -1.0,
            ..valid_request()
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn rating_above_max_invalid() {
        let req = CreateRatingJastiperRequest {
            jastiper_rating: 5.1,
            ..valid_request()
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn review_at_limit_valid() {
        let req = CreateRatingJastiperRequest {
            jastiper_review: Some("a".repeat(1000)),
            ..valid_request()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn review_exceeds_limit_invalid() {
        let req = CreateRatingJastiperRequest {
            jastiper_review: Some("a".repeat(1001)),
            ..valid_request()
        };
        assert!(req.validate().is_err());
    }
}
