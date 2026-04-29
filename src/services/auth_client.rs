use crate::error::AppError;
use crate::services::http_client::internal_post;
use serde_json::json;
use tracing::{debug, error};
use uuid::Uuid;

fn user_url() -> String {
    let url =
        std::env::var("USER_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8082".to_string());
    debug!("🌐 [user] using URL: {}", url);
    url
}

pub(crate) async fn send_jastiper_rating(
    jastiper_id: Uuid,
    order_id: Uuid,
    rating: f64,
    review: Option<&str>,
) -> Result<(), AppError> {
    let url = format!("{}/internal/users/{}/rating", user_url(), jastiper_id,);

    let payload = json!({
        "order_id": order_id,
        "rating":   rating,
    });

    debug!("👤 [user] send_jastiper_rating → POST {}", url);
    debug!(
        "👤 [user] payload: jastiper_id={} order_id={} rating={} review={:?}",
        jastiper_id, order_id, rating, review
    );

    let (status, _) = internal_post(&url, payload).await?;

    debug!("👤 [user] send_jastiper_rating response: HTTP {}", status);

    match status {
        200 => {
            debug!(
                "✅ [user] rating jastiper berhasil dikirim jastiper_id={}",
                jastiper_id
            );
            Ok(())
        }
        404 => {
            debug!(
                "⚠️ [user] jastiper tidak ditemukan jastiper_id={} (non-fatal)",
                jastiper_id
            );
            Ok(())
        }
        409 => {
            debug!(
                "ℹ️ [user] rating sudah dikirim sebelumnya order_id={} (idempotent)",
                order_id
            );
            Ok(())
        }
        code => {
            error!(
                "❌ [user] send_jastiper_rating unexpected status={} jastiper_id={}",
                code, jastiper_id
            );
            Ok(())
        }
    }
}
