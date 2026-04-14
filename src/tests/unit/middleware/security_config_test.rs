#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::security_config::validate_service_key;
    use axum::http::HeaderMap;

    fn make_headers(key: &str, value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::HeaderName::from_bytes(key.as_bytes()).unwrap(),
            axum::http::HeaderValue::from_str(value).unwrap(),
        );
        headers
    }

    #[test]
    fn test_valid_service_key_from_env() {
        temp_env::with_vars([("INTERNAL_SERVICE_KEY", Some("super-secret-key"))], || {
            let headers = make_headers("X-Service-Key", "super-secret-key");
            let result = validate_service_key(&headers);
            assert!(result.is_ok(), "Harusnya OK kalau key cocok dengan env var");
        });
    }

    #[test]
    fn test_invalid_service_key() {
        temp_env::with_vars([("INTERNAL_SERVICE_KEY", Some("correct-key"))], || {
            let headers = make_headers("X-Service-Key", "wrong-key");
            let result = validate_service_key(&headers);
            assert!(result.is_err(), "Harusnya error kalau key salah");

            match result.unwrap_err() {
                crate::error::AppError::Unauthorized(msg) => {
                    assert_eq!(msg, "Invalid service key");
                }
                _ => panic!("Expected AppError::Unauthorized"),
            }
        });
    }

    #[test]
    fn test_missing_service_key_header() {
        temp_env::with_vars([("INTERNAL_SERVICE_KEY", Some("some-key"))], || {
            let headers = HeaderMap::new();
            let result = validate_service_key(&headers);
            assert!(result.is_err(), "Harusnya error kalau header tidak ada");
        });
    }

    #[test]
    fn test_fallback_default_key_when_env_not_set() {
        temp_env::with_vars(
            [("INTERNAL_SERVICE_KEY", None::<&str>)], // unset env var
            || {
                let headers = make_headers("X-Service-Key", "internal-secret");
                let result = validate_service_key(&headers);
                assert!(
                    result.is_ok(),
                    "Harusnya OK dengan fallback default 'internal-secret'"
                );
            },
        );
    }

    #[test]
    fn test_empty_service_key_header_value() {
        temp_env::with_vars([("INTERNAL_SERVICE_KEY", Some("real-key"))], || {
            let headers = make_headers("X-Service-Key", "");
            let result = validate_service_key(&headers);
            assert!(result.is_err(), "Header kosong harusnya gagal validasi");
        });
    }

    #[test]
    fn test_case_sensitive_key_comparison() {
        temp_env::with_vars([("INTERNAL_SERVICE_KEY", Some("MySecretKey"))], || {
            let headers = make_headers("X-Service-Key", "mysecretkey");
            let result = validate_service_key(&headers);
            assert!(result.is_err(), "Perbandingan key harus case-sensitive");
        });
    }
}
