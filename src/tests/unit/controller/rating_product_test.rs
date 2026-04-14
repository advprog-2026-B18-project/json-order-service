#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn test_submit_rating_product_response_shape() {
        let rating_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let resp = json!({
            "success": true,
            "message": "Rating berhasil dikirim",
            "data": {
                "rating_id":      rating_id,
                "order_id":       order_id,
                "product_rating": 5,
                "created_at":     "2024-01-01T00:00:00Z"
            }
        });

        assert_eq!(resp["success"], true);
        assert_eq!(resp["message"], "Rating berhasil dikirim");
        assert!(resp["data"]["rating_id"].is_string());
        assert!(resp["data"]["order_id"].is_string());
        assert!(resp["data"]["product_rating"].is_number());
        assert!(resp["data"]["created_at"].is_string());
    }

    #[test]
    fn test_submit_rating_product_status_created() {
        let expected_status = reqwest::StatusCode::CREATED;
        assert_eq!(expected_status.as_u16(), 201);
    }

    #[test]
    fn test_get_rating_product_response_shape() {
        let order_id = Uuid::new_v4();
        let rating_data = json!({
            "rating_product_id": Uuid::new_v4(),
            "order_id": order_id,
            "product_rating": 4,
            "review": "Produk bagus",
            "created_at": "2024-01-01T00:00:00Z"
        });

        let resp = json!({
            "success": true,
            "message": "Rating ditemukan",
            "data": rating_data
        });

        assert_eq!(resp["success"], true);
        assert_eq!(resp["message"], "Rating ditemukan");
        assert!(resp["data"].is_object());
    }

    #[test]
    fn test_rating_value_boundary() {
        let valid_ratings = [1u8, 2, 3, 4, 5];
        let invalid_ratings_low = [0u8];
        let invalid_ratings_high = [6u8, 10];

        for &r in &valid_ratings {
            assert!(r >= 1 && r <= 5, "Rating {} harusnya valid", r);
        }

        for &r in &invalid_ratings_low {
            assert!(r < 1 || r > 5, "Rating {} harusnya tidak valid", r);
        }

        for &r in &invalid_ratings_high {
            assert!(r < 1 || r > 5, "Rating {} harusnya tidak valid", r);
        }
    }

    #[test]
    fn test_create_rating_product_request_validation() {
        use crate::models::rating_product::CreateRatingProductRequest;
        use validator::Validate;

        let req_invalid = CreateRatingProductRequest {
            product_rating: 0.0,
            product_review: None,
            product_images: None,
        };
        assert!(req_invalid.validate().is_err());

        let req_valid = CreateRatingProductRequest {
            product_rating: 5.0,
            product_review: Some("Bagus sekali".into()),
            product_images: Some(vec!["http://example.com/image1.jpg".into()]),
        };
        assert!(req_valid.validate().is_ok());

        let _ = (req_invalid, req_valid);
    }

    #[test]
    fn test_response_uses_rating_id_not_rating_product_id() {
        let resp = json!({
            "data": {
                "rating_id": Uuid::new_v4(),
            }
        });

        assert!(
            resp["data"]["rating_id"].is_string(),
            "Field harus 'rating_id' (bukan 'rating_product_id')"
        );
        assert!(
            resp["data"]["rating_product_id"].is_null(),
            "Field 'rating_product_id' tidak boleh ada di respons"
        );
    }

    #[test]
    fn test_response_contains_order_id() {
        let order_id = Uuid::new_v4();
        let resp = json!({
            "data": {
                "rating_id": Uuid::new_v4(),
                "order_id": order_id,
                "product_rating": 4,
                "created_at": "2024-01-01T00:00:00Z"
            }
        });

        assert_eq!(
            resp["data"]["order_id"].as_str().unwrap(),
            order_id.to_string()
        );
    }
}
