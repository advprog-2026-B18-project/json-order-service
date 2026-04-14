#[cfg(test)]
mod tests {
    use crate::models::rating_product::CreateRatingProductRequest;
    use validator::Validate;

    fn valid_request() -> CreateRatingProductRequest {
        CreateRatingProductRequest {
            product_rating: 4.0,
            product_review: Some("Produk sesuai deskripsi".to_string()),
            product_images: Some(vec!["https://example.com/img/1.jpg".to_string()]),
        }
    }

    #[test]
    fn valid_request_passes() {
        assert!(valid_request().validate().is_ok());
    }

    #[test]
    fn valid_request_no_review_no_images_passes() {
        let req = CreateRatingProductRequest {
            product_rating: 3.0,
            product_review: None,
            product_images: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn rating_min_boundary_valid() {
        let req = CreateRatingProductRequest {
            product_rating: 1.0,
            ..valid_request()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn rating_max_boundary_valid() {
        let req = CreateRatingProductRequest {
            product_rating: 5.0,
            ..valid_request()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn rating_below_min_invalid() {
        let req = CreateRatingProductRequest {
            product_rating: 0.9,
            ..valid_request()
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn rating_zero_invalid() {
        let req = CreateRatingProductRequest {
            product_rating: 0.0,
            ..valid_request()
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn rating_negative_invalid() {
        let req = CreateRatingProductRequest {
            product_rating: -1.0,
            ..valid_request()
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn rating_above_max_invalid() {
        let req = CreateRatingProductRequest {
            product_rating: 5.1,
            ..valid_request()
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn review_at_limit_valid() {
        let req = CreateRatingProductRequest {
            product_review: Some("a".repeat(1000)),
            ..valid_request()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn review_exceeds_limit_invalid() {
        let req = CreateRatingProductRequest {
            product_review: Some("a".repeat(1001)),
            ..valid_request()
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn images_empty_vec_valid() {
        let req = CreateRatingProductRequest {
            product_images: Some(vec![]),
            ..valid_request()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn images_one_item_valid() {
        let req = CreateRatingProductRequest {
            product_images: Some(vec!["https://example.com/1.jpg".to_string()]),
            ..valid_request()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn images_three_items_valid() {
        let req = CreateRatingProductRequest {
            product_images: Some(vec![
                "https://example.com/1.jpg".to_string(),
                "https://example.com/2.jpg".to_string(),
                "https://example.com/3.jpg".to_string(),
            ]),
            ..valid_request()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn images_four_items_invalid() {
        let req = CreateRatingProductRequest {
            product_images: Some(vec![
                "https://example.com/1.jpg".to_string(),
                "https://example.com/2.jpg".to_string(),
                "https://example.com/3.jpg".to_string(),
                "https://example.com/4.jpg".to_string(),
            ]),
            ..valid_request()
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn images_none_valid() {
        let req = CreateRatingProductRequest {
            product_images: None,
            ..valid_request()
        };
        assert!(req.validate().is_ok());
    }
}
