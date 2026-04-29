use crate::error::AppError;
use serde_json::json;
use tracing::{debug, error, warn};
use uuid::Uuid;

fn inventory_url() -> String {
    let url = std::env::var("INVENTORY_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:8083".to_string());
    debug!("🌐 [inventory] using URL: {}", url);
    url
}

enum StockAction {
    Reserve,
    Release,
}

async fn manage_stock(
    action: StockAction,
    product_id: Uuid,
    order_id: Uuid,
    quantity: i32,
) -> Result<(), AppError> {
    let suffix = match action {
        StockAction::Reserve => "reserve",
        StockAction::Release => "release",
    };

    debug!(
        "📦 [inventory] {} stock product_id={} order_id={} qty={}",
        suffix, product_id, order_id, quantity
    );

    let url = format!(
        "{}/internal/products/{}/stock/{}",
        inventory_url(),
        product_id,
        suffix
    );

    let (status, _) = crate::services::http_client::internal_post(
        &url,
        json!({ "order_id": order_id, "quantity": quantity }),
    )
    .await?;

    match status {
        200 => {
            debug!(
                "✅ [inventory] {} stock berhasil product_id={}",
                suffix, product_id
            );
            Ok(())
        }
        404 => {
            warn!(
                "⚠️ [inventory] produk tidak ditemukan product_id={}",
                product_id
            );
            Err(AppError::NotFound("Produk tidak ditemukan".to_string()))
        }
        409 => {
            warn!(
                "⚠️ [inventory] stok tidak mencukupi product_id={} qty={}",
                product_id, quantity
            );
            Err(AppError::Conflict("Stok tidak mencukupi".to_string()))
        }
        422 => {
            warn!(
                "⚠️ [inventory] produk tidak ACTIVE product_id={}",
                product_id
            );
            Err(AppError::UnprocessableEntity(
                "Produk tidak dalam status ACTIVE".to_string(),
            ))
        }
        code => {
            error!(
                "❌ [inventory] unexpected status={} untuk {} stock product_id={}",
                code, suffix, product_id
            );
            Err(AppError::Internal)
        }
    }
}

pub(crate) async fn reserve_stock(
    product_id: Uuid,
    order_id: Uuid,
    quantity: i32,
) -> Result<(), AppError> {
    debug!(
        "📦 [inventory] reserve_stock product_id={} order_id={} qty={}",
        product_id, order_id, quantity
    );
    manage_stock(StockAction::Reserve, product_id, order_id, quantity).await
}

pub(crate) async fn release_stock(
    product_id: Uuid,
    order_id: Uuid,
    quantity: i32,
) -> Result<(), AppError> {
    debug!(
        "📦 [inventory] release_stock product_id={} order_id={} qty={}",
        product_id, order_id, quantity
    );
    manage_stock(StockAction::Release, product_id, order_id, quantity).await
}

pub(crate) async fn fetch_product(product_id: Uuid) -> Result<serde_json::Value, AppError> {
    let url = format!("{}/products/{}", inventory_url(), product_id);

    debug!("🔍 [inventory] fetch_product → GET {}", url);

    let response = reqwest::Client::new().get(&url).send().await.map_err(|e| {
        error!("❌ [inventory] fetch_product network error: {:?}", e);
        AppError::Internal
    })?;

    let status = response.status().as_u16();
    debug!("🔍 [inventory] fetch_product response: HTTP {}", status);

    match status {
        200 => {
            let body: serde_json::Value = response.json().await.map_err(|e| {
                error!("❌ [inventory] fetch_product parse JSON gagal: {:?}", e);
                AppError::Internal
            })?;
            debug!("✅ [inventory] fetch_product berhasil: {}", body["data"]);
            Ok(body["data"].clone())
        }
        404 => {
            warn!(
                "⚠️ [inventory] produk tidak ditemukan product_id={}",
                product_id
            );
            Err(AppError::NotFound("Produk tidak ditemukan".to_string()))
        }
        422 => {
            warn!(
                "⚠️ [inventory] produk tidak aktif product_id={}",
                product_id
            );
            Err(AppError::UnprocessableEntity(
                "Produk tidak aktif".to_string(),
            ))
        }
        code => {
            error!(
                "❌ [inventory] fetch_product unexpected status={} product_id={}",
                code, product_id
            );
            Err(AppError::Internal)
        }
    }
}

pub(crate) async fn send_product_rating(
    product_id: Uuid,
    order_id: Uuid,
    rating: f64,
    review: Option<&str>,
    product_images: Vec<&str>,
) -> Result<(), AppError> {
    debug!(
        "⭐ [inventory] send_product_rating product_id={} order_id={}",
        product_id, order_id
    );

    let url = format!(
        "{}/internal/products/{}/post-order",
        inventory_url(),
        product_id,
    );

    let payload = json!({
        "order_id":       order_id,
        "action":         "CONFIRM",
        "rating":         rating,
        "review_text":    review,
        "product_images": product_images,
    });

    let (status, _) = crate::services::http_client::internal_post(&url, payload).await?;

    match status {
        200 => {
            debug!(
                "✅ [inventory] product rating terkirim product_id={}",
                product_id
            );
            Ok(())
        }
        404 => {
            debug!(
                "⚠️ [inventory] produk tidak ditemukan product_id={} (non-fatal)",
                product_id
            );
            Ok(())
        }
        409 => {
            debug!(
                "ℹ️ [inventory] rating produk sudah dikirim order_id={} (idempotent)",
                order_id
            );
            Ok(())
        }
        code => {
            error!(
                "❌ [inventory] send_product_rating unexpected status={} product_id={}",
                code, product_id
            );
            Ok(())
        }
    }
}
