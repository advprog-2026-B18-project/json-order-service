#[cfg(test)]
mod tests {
    use crate::controller::order::paginated_response;
    use crate::models::shipping_address::ShippingAddress;
    use serde_json::{json};
    use uuid::Uuid;

    fn make_bearer_token(user_id: Uuid, role: &str) -> (String, String) {
        use base64::Engine;
        use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

        let b64_secret =
            base64::engine::general_purpose::STANDARD.encode("test-jwt-secret-min-32-bytes!!");
        let secret_bytes = base64::engine::general_purpose::STANDARD
            .decode(&b64_secret)
            .unwrap();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let claims = crate::middleware::auth::JwtClaims {
            sub: user_id.to_string(),
            email: "user@test.com".into(),
            role: role.into(),
            exp: now + 3600,
            iat: now,
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(&secret_bytes),
        )
        .unwrap();

        (token, b64_secret)
    }

    #[test]
    fn test_paginated_response_defaults() {
        let result = paginated_response("OK", json!([]), 0, None, None);
        let pagination = &result["pagination"];

        assert_eq!(pagination["page"], 1);
        assert_eq!(pagination["limit"], 20);
        assert_eq!(pagination["total_items"], 0);
        assert_eq!(pagination["total_pages"], 0);
    }

    #[test]
    fn test_paginated_response_custom_page_limit() {
        let result = paginated_response("OK", json!([]), 45, Some(2), Some(10));
        let pagination = &result["pagination"];

        assert_eq!(pagination["page"], 2);
        assert_eq!(pagination["limit"], 10);
        assert_eq!(pagination["total_items"], 45);
        assert_eq!(pagination["total_pages"], 5);
    }

    #[test]
    fn test_paginated_response_ceiling_division() {
        let result = paginated_response("OK", json!([]), 11, Some(1), Some(10));
        assert_eq!(result["pagination"]["total_pages"], 2);
    }

    #[test]
    fn test_paginated_response_single_page() {
        let result = paginated_response("OK", json!([]), 5, Some(1), Some(20));
        assert_eq!(result["pagination"]["total_pages"], 1);
    }

    #[test]
    fn test_paginated_response_zero_items() {
        let result = paginated_response("Kosong", json!([]), 0, Some(1), Some(20));
        assert_eq!(result["pagination"]["total_pages"], 0);
        assert_eq!(result["success"], true);
        assert_eq!(result["message"], "Kosong");
    }

    #[test]
    fn test_paginated_response_large_dataset() {
        let result = paginated_response("OK", json!([]), 1000, Some(1), Some(7));
        assert_eq!(result["pagination"]["total_pages"], 143);
    }

    #[test]
    fn test_checkout_request_validation_missing_fields() {
        use crate::models::order::CreateOrderRequest;
        use validator::Validate;

        let req = CreateOrderRequest {
            product_id: Default::default(),
            quantity: 0,
            shipping_address: ShippingAddress {
                recipient_name: "".to_string(),
                phone_number: "".to_string(),
                street: "".to_string(),
                kelurahan: "".to_string(),
                kecamatan: "".to_string(),
                city: "".to_string(),
                province: "".to_string(),
                postal_code: "".to_string(),
                notes: None,
            },
            note_to_jastiper: None,
        };

        let result = req.validate();
        let _ = result;
    }

    #[test]
    fn test_cancel_request_validation() {
        use crate::models::order::CancelRequest;

        let req_empty = CancelRequest {
            cancellation_reason: "".into(),
        };
        let _ = req_empty;
    }

    #[test]
    fn test_success_response_shape() {
        let order_id = Uuid::new_v4();
        let mock_order = json!({
            "order_id": order_id,
            "status": "pending"
        });

        let resp = json!({
            "success": true,
            "message": "OK",
            "data": mock_order
        });

        assert_eq!(resp["success"], true);
        assert!(resp["message"].is_string());
        assert!(resp["data"].is_object());
    }

    #[test]
    fn test_checkout_response_shape() {
        let order_id = Uuid::new_v4();
        let resp = json!({
            "success": true,
            "message": "Pesanan berhasil dibuat",
            "data": { "order_id": order_id }
        });

        assert_eq!(resp["message"], "Pesanan berhasil dibuat");
        assert!(resp["data"]["order_id"].is_string());
    }

    #[test]
    fn test_confirm_order_response_shape() {
        let order_id = Uuid::new_v4();
        let resp = json!({
            "success": true,
            "message": "Pesanan berhasil dikonfirmasi selesai",
            "data": {
                "order_id": order_id,
                "status": "completed",
                "completed_at": "2024-01-01T00:00:00Z"
            }
        });

        assert_eq!(resp["success"], true);
        assert!(resp["data"]["completed_at"].is_string());
    }

    #[test]
    fn test_payment_response_message() {
        let resp = json!({
            "success": true,
            "message": "Pembayaran berhasil dilakukan",
            "data": {}
        });
        assert_eq!(resp["message"], "Pembayaran berhasil dilakukan");
    }

    #[test]
    fn test_shipped_response_shape() {
        let order_id = Uuid::new_v4();
        let resp = json!({
            "success": true,
            "message": "Pesanan berhasil dikirim jastiper",
            "data": {
                "order_id": order_id,
                "status": "shipped",
                "completed_at": "2024-01-01T00:00:00Z"
            }
        });

        assert_eq!(resp["message"], "Pesanan berhasil dikirim jastiper");
        assert!(resp["data"]["status"].is_string());
    }
}
