use std::sync::Once;

static INIT: Once = Once::new();

fn setup_env() {
    INIT.call_once(|| unsafe {
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-internal-key");
    });
}

// ──────────────────────────────────────────────────────────────
// service_key / env var
// ──────────────────────────────────────────────────────────────

#[test]
fn service_key_terbaca_dari_env() {
    setup_env();
    let key = std::env::var("INTERNAL_SERVICE_KEY").unwrap();
    assert!(!key.is_empty(), "INTERNAL_SERVICE_KEY harus tidak kosong");
}

// ──────────────────────────────────────────────────────────────
// internal_post — gagal karena host tidak bisa dijangkau
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn internal_post_gagal_host_tidak_bisa_dijangkau() {
    setup_env();

    let result = crate::services::http_client::internal_post(
        "http://host-tidak-ada.invalid:9999/test",
        serde_json::json!({"key": "value"}),
    )
    .await;

    assert!(
        result.is_err(),
        "Seharusnya error ketika host tidak bisa dijangkau"
    );
}

#[tokio::test]
async fn internal_post_gagal_url_tidak_valid() {
    setup_env();

    let result =
        crate::services::http_client::internal_post("bukan-url-valid", serde_json::json!({})).await;

    assert!(result.is_err(), "Seharusnya error ketika URL tidak valid");
}

// ──────────────────────────────────────────────────────────────
// internal_get — gagal karena host tidak bisa dijangkau
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn internal_get_gagal_host_tidak_bisa_dijangkau() {
    setup_env();

    let result = crate::services::http_client::internal_get(
        "http://host-tidak-ada.invalid:9999/test",
        serde_json::json!({}),
    )
    .await;

    assert!(
        result.is_err(),
        "Seharusnya error ketika host tidak bisa dijangkau"
    );
}

#[tokio::test]
async fn internal_get_gagal_url_tidak_valid() {
    setup_env();

    let result =
        crate::services::http_client::internal_get("bukan-url-valid", serde_json::json!({})).await;

    assert!(result.is_err(), "Seharusnya error ketika URL tidak valid");
}
