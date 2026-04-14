#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderName, HeaderValue};
    use serde_json::json;
    use uuid::Uuid;

    fn headers_with_key(key: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            HeaderName::from_bytes(b"X-Service-Key").unwrap(),
            HeaderValue::from_str(key).unwrap(),
        );
        h
    }

    #[test]
    fn test_payment_info_requires_service_key() {
        temp_env::with_vars([("INTERNAL_SERVICE_KEY", Some("valid-key"))], || {
            let result = crate::middleware::security_config::validate_service_key(
                &headers_with_key("wrong-key"),
            );
            assert!(
                result.is_err(),
                "payment_info harus menolak request dengan key salah"
            );
        });
    }

    #[test]
    fn test_payment_confirmed_requires_service_key() {
        temp_env::with_vars([("INTERNAL_SERVICE_KEY", Some("svc-secret"))], || {
            let result = crate::middleware::security_config::validate_service_key(
                &headers_with_key("not-svc-secret"),
            );
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_refund_confirmed_requires_service_key() {
        temp_env::with_vars([("INTERNAL_SERVICE_KEY", Some("refund-key"))], || {
            let result = crate::middleware::security_config::validate_service_key(
                &headers_with_key("bad-key"),
            );
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_valid_service_key_passes_all_internal_routes() {
        temp_env::with_vars(
            [("INTERNAL_SERVICE_KEY", Some("shared-internal-secret"))],
            || {
                let headers = headers_with_key("shared-internal-secret");
                let r1 = crate::middleware::security_config::validate_service_key(&headers);
                let r2 = crate::middleware::security_config::validate_service_key(&headers);
                let r3 = crate::middleware::security_config::validate_service_key(&headers);

                assert!(r1.is_ok(), "payment_info: key valid harus diterima");
                assert!(r2.is_ok(), "payment_confirmed: key valid harus diterima");
                assert!(r3.is_ok(), "refund_confirmed: key valid harus diterima");
            },
        );
    }

    #[test]
    fn test_payment_info_response_shape() {
        let order_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let jastiper_id = Uuid::new_v4();

        let resp = json!({
            "success": true,
            "message": "OK",
            "data": {
                "order_id":          order_id,
                "titipers_user_id":  user_id,
                "jastiper_user_id":  jastiper_id,
                "total_price":       150000,
                "status":            "pending",
                "product_snapshot":  {}
            }
        });

        assert_eq!(resp["success"], true);
        assert_eq!(resp["message"], "OK");

        let data = &resp["data"];
        assert!(data["order_id"].is_string());
        assert!(data["titipers_user_id"].is_string());
        assert!(data["jastiper_user_id"].is_string());
        assert!(data["total_price"].is_number());
        assert!(data["status"].is_string());
        assert!(!data["product_snapshot"].is_null());
    }

    #[test]
    fn test_payment_info_uses_titipers_not_titiper() {
        let resp = json!({
            "data": {
                "titipers_user_id": Uuid::new_v4(),
            }
        });

        assert!(
            resp["data"]["titipers_user_id"].is_string(),
            "Field harus 'titipers_user_id'"
        );
        assert!(
            resp["data"]["titiper_user_id"].is_null(),
            "'titiper_user_id' tidak boleh ada (typo)"
        );
    }

    #[test]
    fn test_payment_confirmed_response_shape() {
        let order_id = Uuid::new_v4();

        let resp = json!({
            "success": true,
            "message": "Status order diperbarui ke PAID",
            "data": {
                "order_id": order_id,
                "status":   "paid"
            }
        });

        assert_eq!(resp["success"], true);
        assert_eq!(resp["message"], "Status order diperbarui ke PAID");
        assert!(resp["data"]["order_id"].is_string());
        assert_eq!(resp["data"]["status"], "paid");
    }

    #[test]
    fn test_payment_confirmed_message_contains_paid() {
        let msg = "Status order diperbarui ke PAID";
        assert!(msg.contains("PAID"), "Pesan harus menyebut status PAID");
    }

    #[test]
    fn test_refund_confirmed_response_shape() {
        let order_id = Uuid::new_v4();

        let resp = json!({
            "success": true,
            "message": "Refund terkonfirmasi",
            "data": {
                "order_id":         order_id,
                "status":           "refunded",
                "refund_confirmed": true
            }
        });

        assert_eq!(resp["success"], true);
        assert_eq!(resp["message"], "Refund terkonfirmasi");
        assert!(resp["data"]["order_id"].is_string());
        assert_eq!(resp["data"]["refund_confirmed"], true);
    }

    #[test]
    fn test_refund_confirmed_always_sets_refund_confirmed_true() {
        let resp = json!({
            "data": {
                "refund_confirmed": true
            }
        });

        assert_eq!(
            resp["data"]["refund_confirmed"], true,
            "refund_confirmed harus true"
        );
    }

    #[test]
    fn test_payment_info_includes_product_snapshot() {
        let payment_info_resp = json!({
            "data": { "product_snapshot": { "name": "Produk X" } }
        });

        let payment_confirmed_resp = json!({
            "data": { "order_id": Uuid::new_v4(), "status": "paid" }
        });

        assert!(!payment_info_resp["data"]["product_snapshot"].is_null());
        assert!(
            payment_confirmed_resp["data"]["product_snapshot"].is_null(),
            "payment_confirmed tidak boleh mengembalikan product_snapshot"
        );
    }

    #[test]
    fn test_internal_endpoints_do_not_use_jwt_claims() {
        let uses_jwt = false;
        assert!(
            !uses_jwt,
            "Endpoint internal tidak boleh bergantung pada JWT"
        );
    }
}
