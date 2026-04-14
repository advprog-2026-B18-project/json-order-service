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
}
