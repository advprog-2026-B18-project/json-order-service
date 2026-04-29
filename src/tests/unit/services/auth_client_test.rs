use mockito::Server;
use uuid::Uuid;

fn setup() {
    unsafe {
        std::env::set_var("INTERNAL_SERVICE_KEY", "test-key");
    }
}

#[tokio::test]
async fn send_jastiper_rating_sukses() {
    setup();
    let mut server = Server::new_async().await;
    let jastiper_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("USER_SERVICE_URL", server.url());
    }

    server
        .mock(
            "POST",
            format!("/internal/users/{}/rating", jastiper_id).as_str(),
        )
        .with_status(200)
        .create_async()
        .await;

    let result = crate::services::auth_client::send_jastiper_rating(
        jastiper_id,
        Uuid::new_v4(),
        4.5,
        Some("Jastiper terbaik"),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn send_jastiper_rating_jastiper_tidak_ditemukan_404_non_fatal() {
    setup();
    let mut server = Server::new_async().await;
    let jastiper_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("USER_SERVICE_URL", server.url());
    }

    server
        .mock(
            "POST",
            format!("/internal/users/{}/rating", jastiper_id).as_str(),
        )
        .with_status(404)
        .create_async()
        .await;

    let result =
        crate::services::auth_client::send_jastiper_rating(jastiper_id, Uuid::new_v4(), 4.5, None)
            .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn send_jastiper_rating_idempotent_409() {
    setup();
    let mut server = Server::new_async().await;
    let jastiper_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("USER_SERVICE_URL", server.url());
    }

    server
        .mock(
            "POST",
            format!("/internal/users/{}/rating", jastiper_id).as_str(),
        )
        .with_status(409)
        .create_async()
        .await;

    let result = crate::services::auth_client::send_jastiper_rating(
        jastiper_id,
        Uuid::new_v4(),
        5.0,
        Some("Mantap"),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn send_jastiper_rating_unexpected_status_tetap_ok() {
    setup();
    let mut server = Server::new_async().await;
    let jastiper_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("USER_SERVICE_URL", server.url());
    }

    server
        .mock(
            "POST",
            format!("/internal/users/{}/rating", jastiper_id).as_str(),
        )
        .with_status(500)
        .create_async()
        .await;

    let result =
        crate::services::auth_client::send_jastiper_rating(jastiper_id, Uuid::new_v4(), 3.0, None)
            .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn send_jastiper_rating_tanpa_review_sukses() {
    setup();
    let mut server = Server::new_async().await;
    let jastiper_id = Uuid::new_v4();
    unsafe {
        std::env::set_var("USER_SERVICE_URL", server.url());
    }

    server
        .mock(
            "POST",
            format!("/internal/users/{}/rating", jastiper_id).as_str(),
        )
        .with_status(200)
        .create_async()
        .await;

    let result =
        crate::services::auth_client::send_jastiper_rating(jastiper_id, Uuid::new_v4(), 5.0, None)
            .await;

    assert!(result.is_ok());
}
