#[cfg(test)]
mod tests {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    use std::time::{SystemTime, UNIX_EPOCH};
    use uuid::Uuid;

    fn now_secs() -> usize {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize
    }

    /// Buat JWT valid dengan secret base64-encoded
    fn build_token(claims: &JwtClaims, b64_secret: &str) -> String {
        use base64::{Engine, engine::general_purpose};
        let secret_bytes = general_purpose::STANDARD.decode(b64_secret).unwrap();
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(&secret_bytes),
        )
        .unwrap()
    }

    fn default_claims(extra_secs: usize) -> JwtClaims {
        JwtClaims {
            sub: Uuid::new_v4().to_string(),
            email: "test@example.com".into(),
            role: "buyer".into(),
            exp: now_secs() + extra_secs,
            iat: now_secs(),
        }
    }

    #[test]
    fn test_user_id_valid_uuid() {
        let id = Uuid::new_v4();
        let claims = JwtClaims {
            sub: id.to_string(),
            email: "".into(),
            role: "buyer".into(),
            exp: now_secs() + 3600,
            iat: now_secs(),
        };

        let result = claims.user_id();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), id);
    }

    #[test]
    fn test_user_id_invalid_uuid() {
        let claims = JwtClaims {
            sub: "bukan-uuid".into(),
            email: "".into(),
            role: "buyer".into(),
            exp: now_secs() + 3600,
            iat: now_secs(),
        };

        let result = claims.user_id();
        assert!(result.is_err(), "Sub yang bukan UUID harus error");
    }

    #[test]
    fn test_user_id_empty_sub() {
        let claims = JwtClaims {
            sub: "".into(),
            email: "".into(),
            role: "buyer".into(),
            exp: now_secs() + 3600,
            iat: now_secs(),
        };

        assert!(claims.user_id().is_err());
    }

    #[test]
    fn test_role_valid_buyer() {
        let claims = JwtClaims {
            sub: Uuid::new_v4().to_string(),
            email: "".into(),
            role: "TITIPERS".into(),
            exp: now_secs() + 3600,
            iat: now_secs(),
        };

        let result = claims.role();
        assert!(result.is_ok());
    }

    #[test]
    fn test_role_valid_jastiper() {
        let claims = JwtClaims {
            sub: Uuid::new_v4().to_string(),
            email: "".into(),
            role: "JASTIPER".into(),
            exp: now_secs() + 3600,
            iat: now_secs(),
        };

        let result = claims.role();
        assert!(result.is_ok());
    }

    #[test]
    fn test_role_invalid_string() {
        let claims = JwtClaims {
            sub: Uuid::new_v4().to_string(),
            email: "".into(),
            role: "superadmin-tidak-ada".into(),
            exp: now_secs() + 3600,
            iat: now_secs(),
        };

        let result = claims.role();
        assert!(result.is_err(), "Role tidak dikenal harus error");
        match result.unwrap_err() {
            crate::error::AppError::Unauthorized(msg) => {
                assert!(msg.contains("Role tidak valid"));
            }
            _ => panic!("Expected AppError::Unauthorized"),
        }
    }

    #[test]
    fn test_role_empty_string() {
        let claims = JwtClaims {
            sub: Uuid::new_v4().to_string(),
            email: "".into(),
            role: "".into(),
            exp: now_secs() + 3600,
            iat: now_secs(),
        };

        assert!(claims.role().is_err());
    }

    use crate::middleware::auth::JwtClaims;
    use axum::extract::FromRequestParts;
    use axum::http::Request;
    use base64::Engine;

    async fn extract_claims(req: Request<()>) -> Result<JwtClaims, crate::error::AppError> {
        let (mut parts, _) = req.into_parts();
        JwtClaims::from_request_parts(&mut parts, &()).await
    }

    #[tokio::test]
    async fn test_from_request_parts_valid_token() {
        let b64_secret = base64::engine::general_purpose::STANDARD
            .encode("my-test-secret-key-at-least-32-bytes!!");

        let claims = default_claims(3600);
        let token = build_token(&claims, &b64_secret);

        temp_env::async_with_vars([("JWT_SECRET", Some(b64_secret.as_str()))], async {
            let req = Request::builder()
                .header("Authorization", format!("Bearer {}", token))
                .body(())
                .unwrap();

            let result = extract_claims(req).await;
            assert!(result.is_ok(), "Token valid harus berhasil diekstrak");
            assert_eq!(result.unwrap().sub, claims.sub);
        })
        .await;
    }

    #[tokio::test]
    async fn test_from_request_parts_missing_auth_header() {
        let req = Request::builder().body(()).unwrap();
        let result = extract_claims(req).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::AppError::Unauthorized(msg) => {
                assert!(msg.contains("Authorization"));
            }
            _ => panic!("Expected Unauthorized"),
        }
    }

    #[tokio::test]
    async fn test_from_request_parts_wrong_prefix() {
        let req = Request::builder()
            .header("Authorization", "Token abc123") // harus Bearer
            .body(())
            .unwrap();

        let result = extract_claims(req).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::AppError::Unauthorized(msg) => {
                assert!(msg.contains("Bearer"));
            }
            _ => panic!("Expected Unauthorized"),
        }
    }

    #[tokio::test]
    async fn test_from_request_parts_expired_token() {
        let b64_secret = base64::engine::general_purpose::STANDARD
            .encode("my-test-secret-key-at-least-32-bytes!!");

        // exp di masa lalu
        let mut claims = default_claims(0);
        claims.exp = now_secs() - 3600;
        let token = build_token(&claims, &b64_secret);

        temp_env::async_with_vars([("JWT_SECRET", Some(b64_secret.as_str()))], async {
            let req = Request::builder()
                .header("Authorization", format!("Bearer {}", token))
                .body(())
                .unwrap();

            let result = extract_claims(req).await;
            assert!(result.is_err(), "Token expired harus ditolak");
        })
        .await;
    }

    #[tokio::test]
    async fn test_from_request_parts_wrong_secret() {
        let signing_secret = base64::engine::general_purpose::STANDARD
            .encode("signing-secret-key-at-least-32-bytes!!");
        let verifying_secret =
            base64::engine::general_purpose::STANDARD.encode("different-secret-key-at-least-32!!!");

        let claims = default_claims(3600);
        let token = build_token(&claims, &signing_secret);

        temp_env::async_with_vars([("JWT_SECRET", Some(verifying_secret.as_str()))], async {
            let req = Request::builder()
                .header("Authorization", format!("Bearer {}", token))
                .body(())
                .unwrap();

            let result = extract_claims(req).await;
            assert!(result.is_err(), "Token dengan secret berbeda harus ditolak");
        })
        .await;
    }

    #[tokio::test]
    async fn test_from_request_parts_malformed_token() {
        let secret =
            base64::engine::general_purpose::STANDARD.encode("any-secret-32-bytes-minimum!!!!!");

        temp_env::async_with_vars([("JWT_SECRET", Some(secret.as_str()))], async {
            let req = Request::builder()
                .header("Authorization", "Bearer ini.bukan.token.valid")
                .body(())
                .unwrap();

            let result = extract_claims(req).await;
            assert!(result.is_err());
        })
        .await;
    }

    #[tokio::test]
    async fn test_from_request_parts_invalid_base64_secret() {
        temp_env::async_with_vars(
            [("JWT_SECRET", Some("!!!bukan_base64_valid!!!###"))],
            async {
                let req = Request::builder()
                    .header("Authorization", "Bearer some.jwt.token")
                    .body(())
                    .unwrap();

                let result = extract_claims(req).await;
                assert!(result.is_err());
                match result.unwrap_err() {
                    crate::error::AppError::Unauthorized(msg) => {
                        assert!(
                            msg.contains("JWT_SECRET") || msg.contains("Token"),
                            "Pesan error: {}",
                            msg
                        );
                    }
                    _ => panic!("Expected Unauthorized"),
                }
            },
        )
        .await;
    }

    // === Base64 padding edge cases ===

    #[tokio::test]
    async fn test_from_request_parts_secret_len_2mod4_padded_to_ab() {
        // JWT_SECRET has b64 length 2 mod 4 → triggers "==" padding
        temp_env::async_with_vars([("JWT_SECRET", Some("YQ"))], async {
            let req = Request::builder()
                .header("Authorization", "Bearer some.jwt.token")
                .body(())
                .unwrap();
            let result = extract_claims(req).await;
            // Decoding will fail because token doesn't match, but padding logic executes
            assert!(result.is_err());
        })
        .await;
    }

    #[tokio::test]
    async fn test_from_request_parts_secret_len_3mod4_padded_to_abc() {
        // JWT_SECRET has b64 length 3 mod 4 → triggers padding by adding "="
        temp_env::async_with_vars([("JWT_SECRET", Some("YWI"))], async {
            let req = Request::builder()
                .header("Authorization", "Bearer some.jwt.token")
                .body(())
                .unwrap();
            let result = extract_claims(req).await;
            assert!(result.is_err());
        })
        .await;
    }

    #[tokio::test]
    async fn test_from_request_parts_secret_already_padded() {
        // JWT_SECRET is already valid base64 with proper length
        let b64_secret = base64::engine::general_purpose::STANDARD
            .encode("my-test-secret-key-at-least-32-bytes!!");
        let token = build_token(&default_claims(3600), &b64_secret);

        temp_env::async_with_vars([("JWT_SECRET", Some(b64_secret.as_str()))], async move {
            let req = Request::builder()
                .header("Authorization", format!("Bearer {}", token))
                .body(())
                .unwrap();
            let result = extract_claims(req).await;
            assert!(result.is_ok());
        })
        .await;
    }

    #[warn(deprecated)]
    #[tokio::test]
    async fn test_from_request_parts_secret_ends_with_double_equal_adjusts_char() {
        // Use a 2-char b64 string that after padding becomes "xx=="
        // "YQ" → clean len=2 → padded to "YQ==" → triggers adjustment
        let raw_secret = base64::engine::general_purpose::STANDARD
            .decode("YQ==")
            .unwrap();
        let signing_secret = base64::engine::general_purpose::STANDARD.encode(&raw_secret);
        let token = build_token(&default_claims(3600), &signing_secret);

        temp_env::async_with_vars([("JWT_SECRET", Some("YQ"))], async move {
            let req = Request::builder()
                .header("Authorization", format!("Bearer {}", token))
                .body(())
                .unwrap();
            let result = extract_claims(req).await;
            assert!(result.is_ok());
        })
        .await;
    }

    #[tokio::test]
    async fn test_from_request_parts_secret_with_default_change_me() {
        // JWT_SECRET is not set → falls back to "change-me"
        temp_env::async_with_vars([("JWT_SECRET", None::<&str>)], async {
            let req = Request::builder()
                .header("Authorization", "Bearer some.jwt.token")
                .body(())
                .unwrap();
            let result = extract_claims(req).await;
            // "change-me" has 9 chars, 9 % 4 = 1 → no padding
            assert!(result.is_err());
        })
        .await;
    }

    // === From-parts tests (extracted from inline #[cfg(test)] in auth.rs) ===
    use axum::http::request::Parts;

    fn make_parts(auth_header: Option<&str>) -> Parts {
        let mut builder = Request::builder();
        if let Some(value) = auth_header {
            builder = builder.header("Authorization", value);
        }
        let (parts, _) = builder.body(()).unwrap().into_parts();
        parts
    }

    #[tokio::test]
    async fn test_jwt_claims_missing_auth_header() {
        let mut parts = make_parts(None);
        let result = JwtClaims::from_request_parts(&mut parts, &()).await;
        assert!(
            matches!(result, Err(crate::error::AppError::Unauthorized(msg)) if msg.contains("tidak ditemukan"))
        );
    }

    #[tokio::test]
    async fn test_jwt_claims_bad_format_no_bearer_prefix() {
        let mut parts = make_parts(Some("InvalidToken"));
        let result = JwtClaims::from_request_parts(&mut parts, &()).await;
        assert!(
            matches!(result, Err(crate::error::AppError::Unauthorized(msg)) if msg.contains("Format token"))
        );
    }

    #[tokio::test]
    async fn test_jwt_claims_bad_format_typo_bearer() {
        let mut parts = make_parts(Some("Bearr token"));
        let result = JwtClaims::from_request_parts(&mut parts, &()).await;
        assert!(
            matches!(result, Err(crate::error::AppError::Unauthorized(msg)) if msg.contains("Format token"))
        );
    }

    #[tokio::test]
    async fn test_jwt_claims_invalid_token_signature() {
        temp_env::async_with_vars(&[("JWT_SECRET", Some("dGVzdA"))], async {
            let mut parts = make_parts(Some("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwicm9sZSI6IkN1c3RvbWVyIn0.invalid"));
            let result = JwtClaims::from_request_parts(&mut parts, &()).await;
            assert!(matches!(result, Err(crate::error::AppError::Unauthorized(msg)) if msg.contains("Token tidak valid")));
        }).await;
    }

    #[tokio::test]
    async fn test_jwt_claims_invalid_base64_secret() {
        temp_env::async_with_vars(&[("JWT_SECRET", Some("!!!"))], async {
            let mut parts = make_parts(Some("Bearer dGVzdA"));
            let result = JwtClaims::from_request_parts(&mut parts, &()).await;
            assert!(matches!(result, Err(crate::error::AppError::Unauthorized(msg)) if msg.contains("JWT_SECRET tidak valid")));
        }).await;
    }

    #[tokio::test]
    async fn test_jwt_claims_valid_token_success() {
        temp_env::async_with_vars(&[("JWT_SECRET", Some("dGVzdA"))], async {
            let user_id = Uuid::new_v4();
            let claims = JwtClaims {
                sub: user_id.to_string(),
                email: "test@test.com".to_string(),
                role: "Customer".to_string(),
                exp: (chrono::Utc::now() + chrono::Duration::minutes(60)).timestamp() as usize,
                iat: chrono::Utc::now().timestamp() as usize,
            };
            let token = jsonwebtoken::encode(
                &jsonwebtoken::Header::default(),
                &claims,
                &jsonwebtoken::EncodingKey::from_secret(b"test"),
            )
            .unwrap();

            let mut parts = make_parts(Some(&format!("Bearer {}", token)));
            let result = JwtClaims::from_request_parts(&mut parts, &()).await;

            assert!(result.is_ok());
            assert_eq!(result.unwrap().user_id().unwrap(), user_id);
        })
        .await;
    }
}
