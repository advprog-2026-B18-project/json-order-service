use mockito::Server;
use serde_json::json;

fn set_service_key() {
    unsafe {
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }
}

#[tokio::test]
async fn internal_post_sukses_return_status_200() {
    set_service_key();
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/test-endpoint")
        .with_status(200)
        .create_async()
        .await;

    let url = format!("{}/test-endpoint", server.url());
    let result = crate::services::http_client::internal_post(&url, json!({"key": "val"})).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 200);
    mock.assert_async().await;
}

#[tokio::test]
async fn internal_post_meneruskan_status_404() {
    set_service_key();
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/test-endpoint")
        .with_status(404)
        .create_async()
        .await;

    let url = format!("{}/test-endpoint", server.url());
    let result = crate::services::http_client::internal_post(&url, json!({})).await;

    assert_eq!(result.unwrap(), 404);
    mock.assert_async().await;
}

#[tokio::test]
async fn internal_post_meneruskan_status_409() {
    set_service_key();
    let mut server = Server::new_async().await;

    server
        .mock("POST", "/test-endpoint")
        .with_status(409)
        .create_async()
        .await;

    let url = format!("{}/test-endpoint", server.url());
    let result = crate::services::http_client::internal_post(&url, json!({})).await;

    assert_eq!(result.unwrap(), 409);
}

#[tokio::test]
async fn internal_post_gagal_network_error() {
    set_service_key();
    let result =
        crate::services::http_client::internal_post("http://localhost:0/tidak-ada", json!({}))
            .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn internal_post_mengirim_header_service_key() {
    set_service_key();
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/cek-header")
        .match_header("X-Service-Key", "test-key")
        .with_status(200)
        .create_async()
        .await;

    let url = format!("{}/cek-header", server.url());
    let _ = crate::services::http_client::internal_post(&url, json!({})).await;

    mock.assert_async().await;
}

#[tokio::test]
async fn internal_get_sukses_return_status_dan_body() {
    set_service_key();
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/test-get")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"is_sufficient": true}"#)
        .create_async()
        .await;

    let url = format!("{}/test-get", server.url());
    let result = crate::services::http_client::internal_get(&url, json!({})).await;

    assert!(result.is_ok());
    let (status, body) = result.unwrap();
    assert_eq!(status, 200);
    assert_eq!(body["is_sufficient"], true);
    mock.assert_async().await;
}

#[tokio::test]
async fn internal_get_gagal_body_bukan_json() {
    set_service_key();
    let mut server = Server::new_async().await;

    server
        .mock("GET", "/invalid-json")
        .with_status(200)
        .with_body("bukan json sama sekali")
        .create_async()
        .await;

    let url = format!("{}/invalid-json", server.url());
    let result = crate::services::http_client::internal_get(&url, json!({})).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn internal_get_gagal_network_error() {
    set_service_key();
    let result =
        crate::services::http_client::internal_get("http://localhost:0/tidak-ada", json!({})).await;

    assert!(result.is_err());
}
