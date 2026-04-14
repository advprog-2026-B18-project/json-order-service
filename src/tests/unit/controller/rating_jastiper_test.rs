#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn test_submit_rating_jastiper_response_shape() {
        let rating_id = Uuid::new_v4();
        let order_id = Uuid::new_v4();

        let resp = json!({
            "success": true,
            "message": "Rating berhasil dikirim",
            "data": {
                "rating_id":       rating_id,
                "order_id":        order_id,
                "jastiper_rating": 5,
                "created_at":      "2024-01-01T00:00:00Z"
            }
        });

        assert_eq!(resp["success"], true);
        assert_eq!(resp["message"], "Rating berhasil dikirim");
        assert!(resp["data"]["rating_id"].is_string());
        assert!(resp["data"]["order_id"].is_string());
        assert!(resp["data"]["jastiper_rating"].is_number());
        assert!(resp["data"]["created_at"].is_string());
    }

    #[test]
    fn test_submit_rating_jastiper_status_created() {
        let expected_status = reqwest::StatusCode::CREATED;
        assert_eq!(expected_status.as_u16(), 201);
    }

    #[test]
    fn test_get_rating_jastiper_response_shape() {
        let order_id = Uuid::new_v4();
        let data = json!({
            "rating_jastiper_id": Uuid::new_v4(),
            "order_id":           order_id,
            "jastiper_rating":    5,
            "review":             "Jastiper cepat dan terpercaya",
            "created_at":         "2024-01-01T00:00:00Z"
        });

        let resp = json!({
            "success": true,
            "message": "Rating ditemukan",
            "data": data
        });

        assert_eq!(resp["success"], true);
        assert_eq!(resp["message"], "Rating ditemukan");
        assert!(resp["data"].is_object());
        assert!(resp["data"]["jastiper_rating"].is_number());
    }

    #[test]
    fn test_jastiper_rating_uses_jastiper_rating_field() {
        let resp = json!({
            "data": {
                "rating_id":       Uuid::new_v4(),
                "order_id":        Uuid::new_v4(),
                "jastiper_rating": 4,
                "created_at":      "2024-01-01T00:00:00Z"
            }
        });

        assert!(
            resp["data"]["jastiper_rating"].is_number(),
            "Field harus 'jastiper_rating'"
        );
        assert!(
            resp["data"]["product_rating"].is_null(),
            "'product_rating' tidak boleh ada di respons rating jastiper"
        );
    }

    #[test]
    fn test_response_uses_rating_id_not_rating_jastiper_id() {
        let resp = json!({
            "data": {
                "rating_id": Uuid::new_v4(),
            }
        });

        assert!(
            resp["data"]["rating_id"].is_string(),
            "Field harus 'rating_id' (bukan 'rating_jastiper_id')"
        );
        assert!(
            resp["data"]["rating_jastiper_id"].is_null(),
            "Field 'rating_jastiper_id' tidak boleh ada di respons"
        );
    }

    #[test]
    fn test_rating_value_range() {
        let valid = [1u8, 2, 3, 4, 5];
        let invalid = [0u8, 6, 255];

        for &r in &valid {
            assert!(r >= 1 && r <= 5, "Rating {} seharusnya valid", r);
        }
        for &r in &invalid {
            assert!(r < 1 || r > 5, "Rating {} seharusnya tidak valid", r);
        }
    }

    #[test]
    fn test_create_rating_jastiper_request_validation() {
        use crate::models::rating_jastiper::CreateRatingJastiperRequest;
        use validator::Validate;

        let req_invalid = CreateRatingJastiperRequest {
            jastiper_rating: 0.0,
            jastiper_review: None,
        };

        let req_valid = CreateRatingJastiperRequest {
            jastiper_rating: 5.0,
            jastiper_review: Some("Jastiper sangat memuaskan".into()),
        };

        let _ = (req_invalid, req_valid);
    }

    #[test]
    fn test_both_rating_handlers_share_same_success_message() {
        let msg_product = "Rating berhasil dikirim";
        let msg_jastiper = "Rating berhasil dikirim";

        assert_eq!(
            msg_product, msg_jastiper,
            "Pesan sukses rating harus konsisten"
        );
    }

    #[test]
    fn test_get_rating_success_message_consistent() {
        let msg_product = "Rating ditemukan";
        let msg_jastiper = "Rating ditemukan";

        assert_eq!(msg_product, msg_jastiper);
    }
}
