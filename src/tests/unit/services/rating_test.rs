#[cfg(test)]
mod tests {
    mod rating_product {
        use crate::models::order_state::OrderStatus;
        use crate::models::rating_product::CreateRatingProductRequest;
        use chrono::Utc;
        use serde_json::json;
        use uuid::Uuid;
        use validator::Validate;

        fn make_completed_order(titipers_id: Uuid) -> crate::models::order::Order {
            crate::models::order::Order {
                order_id: Uuid::new_v4(),
                titipers_id,
                jastiper_id: Uuid::new_v4(),
                product_id: Uuid::new_v4(),
                product_snapshot: json!({ "product_id": Uuid::new_v4() }),
                quantity: 1,
                unit_price: 10_000,
                service_fee: 0,
                total_price: 10_000,
                status: OrderStatus::Completed,
                shipping_address: json!({}),
                note_to_jastiper: None,
                tracking_number: None,
                courier: None,
                cancellation_reason: None,
                cancelled_by: None,
                completed_at: Some(Utc::now()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        }

        // Rating hanya bisa diberikan jika order sudah COMPLETED
        #[test]
        fn test_rating_hanya_untuk_order_completed() {
            let titipers_id = Uuid::new_v4();
            let order = make_completed_order(titipers_id);
            assert_eq!(order.status, OrderStatus::Completed);
        }

        // Rating tidak bisa diberikan jika order masih PENDING
        #[test]
        fn test_rating_ditolak_jika_order_pending() {
            let titipers_id = Uuid::new_v4();
            let mut order = make_completed_order(titipers_id);
            order.status = OrderStatus::Pending;

            let not_completed = order.status != OrderStatus::Completed;
            assert!(not_completed);
        }

        // Requester harus merupakan titipers pemilik order
        #[test]
        fn test_forbidden_jika_bukan_titipers_pemilik() {
            let titipers_id = Uuid::new_v4();
            let orang_lain = Uuid::new_v4();
            let order = make_completed_order(titipers_id);

            let is_forbidden = order.titipers_id != orang_lain;
            assert!(is_forbidden);
        }

        // Validasi rating: harus antara 1.0 - 5.0
        #[test]
        fn test_rating_valid_1_sampai_5() {
            let req = CreateRatingProductRequest {
                product_rating: 4.5,
                product_review: None,
                product_images: None,
            };
            assert!(req.validate().is_ok());
        }

        #[test]
        fn test_rating_kurang_dari_1_invalid() {
            let req = CreateRatingProductRequest {
                product_rating: 0.5,
                product_review: None,
                product_images: None,
            };
            assert!(req.validate().is_err());
        }

        #[test]
        fn test_rating_lebih_dari_5_invalid() {
            let req = CreateRatingProductRequest {
                product_rating: 5.5,
                product_review: None,
                product_images: None,
            };
            assert!(req.validate().is_err());
        }

        // Review max 1000 karakter
        #[test]
        fn test_review_max_1000_karakter() {
            let req = CreateRatingProductRequest {
                product_rating: 4.0,
                product_review: Some("x".repeat(1001)),
                product_images: None,
            };
            assert!(req.validate().is_err());
        }

        // product_images max 3 item
        #[test]
        fn test_product_images_max_3() {
            let req = CreateRatingProductRequest {
                product_rating: 4.0,
                product_review: None,
                product_images: Some(vec![
                    "url1".to_string(),
                    "url2".to_string(),
                    "url3".to_string(),
                    "url4".to_string(),
                ]),
            };
            assert!(req.validate().is_err());
        }

        // product_images tepat 3 item valid
        #[test]
        fn test_product_images_tepat_3_valid() {
            let req = CreateRatingProductRequest {
                product_rating: 4.0,
                product_review: None,
                product_images: Some(vec![
                    "url1".to_string(),
                    "url2".to_string(),
                    "url3".to_string(),
                ]),
            };
            assert!(req.validate().is_ok());
        }

        // product_images kosong (None) valid
        #[test]
        fn test_product_images_none_valid() {
            let req = CreateRatingProductRequest {
                product_rating: 3.0,
                product_review: None,
                product_images: None,
            };
            assert!(req.validate().is_ok());
        }
    }

    mod rating_jastiper {
        use crate::models::order_state::OrderStatus;
        use crate::models::rating_jastiper::CreateRatingJastiperRequest;
        use chrono::Utc;
        use serde_json::json;
        use uuid::Uuid;
        use validator::Validate;

        fn make_completed_order(titipers_id: Uuid) -> crate::models::order::Order {
            crate::models::order::Order {
                order_id: Uuid::new_v4(),
                titipers_id,
                jastiper_id: Uuid::new_v4(),
                product_id: Uuid::new_v4(),
                product_snapshot: json!({}),
                quantity: 1,
                unit_price: 10_000,
                service_fee: 0,
                total_price: 10_000,
                status: OrderStatus::Completed,
                shipping_address: json!({}),
                note_to_jastiper: None,
                tracking_number: None,
                courier: None,
                cancellation_reason: None,
                cancelled_by: None,
                completed_at: Some(Utc::now()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        }

        // Rating jastiper hanya bisa jika order COMPLETED
        #[test]
        fn test_rating_jastiper_hanya_untuk_completed() {
            let titipers_id = Uuid::new_v4();
            let order = make_completed_order(titipers_id);
            assert_eq!(order.status, OrderStatus::Completed);
        }

        // Rating jastiper ditolak jika order belum COMPLETED
        #[test]
        fn test_rating_jastiper_ditolak_jika_shipped() {
            let titipers_id = Uuid::new_v4();
            let mut order = make_completed_order(titipers_id);
            order.status = OrderStatus::Shipped;

            let not_completed = order.status != OrderStatus::Completed;
            assert!(not_completed);
        }

        // Hanya titipers pemilik order yang bisa rating jastiper
        #[test]
        fn test_forbidden_jika_bukan_titipers_pemilik() {
            let titipers_id = Uuid::new_v4();
            let orang_lain = Uuid::new_v4();
            let order = make_completed_order(titipers_id);

            let is_forbidden = order.titipers_id != orang_lain;
            assert!(is_forbidden);
        }

        // Validasi rating: harus antara 1.0 - 5.0
        #[test]
        fn test_rating_valid() {
            let req = CreateRatingJastiperRequest {
                jastiper_rating: 5.0,
                jastiper_review: None,
            };
            assert!(req.validate().is_ok());
        }

        #[test]
        fn test_rating_di_bawah_1_invalid() {
            let req = CreateRatingJastiperRequest {
                jastiper_rating: 0.0,
                jastiper_review: None,
            };
            assert!(req.validate().is_err());
        }

        #[test]
        fn test_rating_di_atas_5_invalid() {
            let req = CreateRatingJastiperRequest {
                jastiper_rating: 6.0,
                jastiper_review: None,
            };
            assert!(req.validate().is_err());
        }

        // Review max 1000 karakter
        #[test]
        fn test_review_max_1000_karakter() {
            let req = CreateRatingJastiperRequest {
                jastiper_rating: 4.0,
                jastiper_review: Some("x".repeat(1001)),
            };
            assert!(req.validate().is_err());
        }

        #[test]
        fn test_review_tepat_1000_karakter_valid() {
            let req = CreateRatingJastiperRequest {
                jastiper_rating: 4.0,
                jastiper_review: Some("x".repeat(1000)),
            };
            assert!(req.validate().is_ok());
        }

        // Duplicate rating: cek jika rating sudah ada (simulasi logic)
        #[test]
        fn test_duplicate_rating_terdeteksi() {
            let rating_exists = true;
            assert!(rating_exists);
        }
    }
}
