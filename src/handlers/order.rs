use crate::error::AppError;
use crate::middleware::auth::JwtClaims;
use crate::models::order::{CreateOrderRequest, OrderFilter, PaginationParams};
use crate::repositories::order as repo;
use axum::Json;
use axum::extract::{Path, Query, State};
use reqwest::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use validator::Validate;

pub(crate) async fn reserve_stock(
    product_id: Uuid,
    order_id: Uuid,
    quantity: i32,
) -> Result<(), AppError> {
    let inventory_url = std::env::var("INVENTORY_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:8081".to_string());

    let service_key =
        std::env::var("INTERNAL_SERVICE_KEY").expect("INTERNAL_SERVICE_KEY harus diset di .env");

    let url = format!(
        "{}/internal/products/{}/stock/reserve",
        inventory_url, product_id
    );

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("X-Service-Key", service_key)
        .json(&serde_json::json!({
            "order_id": order_id,
            "quantity": quantity,
        }))
        .send()
        .await
        .map_err(|_| AppError::Internal)?;

    match response.status().as_u16() {
        200 => Ok(()),
        404 => Err(AppError::NotFound("Produk tidak ditemukan".to_string())),
        409 => Err(AppError::Conflict("Stok tidak mencukupi".to_string())),
        422 => Err(AppError::UnprocessableEntity(
            "Produk tidak dalam status ACTIVE".to_string(),
        )),
        _ => Err(AppError::Internal),
    }
}

pub(crate) async fn release_stock(
    product_id: Uuid,
    order_id: Uuid,
    quantity: i32,
) -> Result<(), AppError> {
    let inventory_url = std::env::var("INVENTORY_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:8081".to_string());

    let service_key =
        std::env::var("INTERNAL_SERVICE_KEY").expect("INTERNAL_SERVICE_KEY harus diset di .env");

    let url = format!(
        "{}/internal/products/{}/stock/release",
        inventory_url, product_id
    );

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("X-Service-Key", service_key)
        .json(&serde_json::json!({
            "order_id": order_id,
            "quantity": quantity,
        }))
        .send()
        .await
        .map_err(|_| AppError::Internal)?;

    match response.status().as_u16() {
        200 => Ok(()),
        404 => Err(AppError::NotFound("Produk tidak ditemukan".to_string())),
        409 => Err(AppError::Conflict("Stok tidak mencukupi".to_string())),
        422 => Err(AppError::UnprocessableEntity(
            "Produk tidak dalam status ACTIVE".to_string(),
        )),
        _ => Err(AppError::Internal),
    }
}

pub(crate) async fn deduct_wallet(
    user_id: Uuid,
    order_id: Uuid,
    amount: i64,
    description: &str,
) -> Result<(), AppError> {
    let wallet_url =
        std::env::var("WALLET_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8082".to_string());

    let service_key =
        std::env::var("INTERNAL_SERVICE_KEY").expect("INTERNAL_SERVICE_KEY harus diset di .env");

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/internal/wallets/deduct", wallet_url))
        .header("X-Service-Key", service_key)
        .json(&serde_json::json!({
            "user_id":     user_id,
            "order_id":    order_id,
            "amount":      amount,
            "description": description,
        }))
        .send()
        .await
        .map_err(|_| AppError::Internal)?;

    match response.status().as_u16() {
        200 => Ok(()),
        404 => Err(AppError::NotFound("User tidak ditemukan".to_string())),
        // 409 = sudah diproses (idempotent), anggap sukses
        409 => Ok(()),
        422 => Err(AppError::UnprocessableEntity(
            "Saldo tidak mencukupi".to_string(),
        )),
        _ => Err(AppError::Internal),
    }
}

pub(crate) async fn fetch_product(product_id: Uuid) -> Result<serde_json::Value, AppError> {
    let inventory_url = std::env::var("INVENTORY_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:8083".to_string());

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/products/{}", inventory_url, product_id))
        .send()
        .await
        .map_err(|_| AppError::Internal)?;

    match response.status().as_u16() {
        200 => {
            let body: serde_json::Value = response.json().await.map_err(|_| AppError::Internal)?;
            Ok(body["data"].clone())
        }
        404 => Err(AppError::NotFound("Produk tidak ditemukan".to_string())),
        422 => Err(AppError::UnprocessableEntity(
            "Produk tidak aktif".to_string(),
        )),
        _ => Err(AppError::Internal),
    }
}

// --- POST /orders ---
#[utoipa::path(
    post, path = "/orders",
    tag = "Orders",
    request_body = CreateOrderRequest,
    responses(
        (status=201, description="Pesanan berhasil dibuat", body=Order),
        (status=400, description="Data tidak valid"),
        (status=401, description="Token tidak valid atau tidak ada"),
        (status=403, description="Jastiper mencoba beli produk sendiri"),
        (status=409, description="Stok tidak mencukupi"),
        (status=422, description="Saldo tidak cukup / produk tidak aktif"),
    )
)]
pub async fn checkout(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims, // ← ekstrak dari Bearer token
    Json(req): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    req.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Identitas titipers dari JWT (sub = accountId)
    let titipers_id = claims.user_id()?;
    let order_id = Uuid::new_v4();

    let product = fetch_product(req.product_id).await?;

    let jastiper_id: Uuid =
        serde_json::from_value(product["jastiper_id"].clone()).map_err(|_| AppError::Internal)?;

    // Titipers tidak boleh membeli produknya sendiri
    if titipers_id == jastiper_id {
        return Err(AppError::Forbidden(
            "Jastiper tidak dapat membeli produk milik sendiri".to_string(),
        ));
    }

    let unit_price = product["price"].as_i64().unwrap_or(0);
    let service_fee = product["service_fee"].as_i64().unwrap_or(0);
    let total_price = (unit_price + service_fee) * req.quantity as i64;

    let product_snapshot = json!({
        "product_id":     req.product_id,
        "name":           product["name"],
        "description":    product["description"],
        "image_url":      product["images"][0],
        "origin_country": product["origin_country"],
        "purchase_date":  product["purchase_date"],
        "unit_price":     unit_price,
        "service_fee":    service_fee,
    });

    reserve_stock(req.product_id, order_id, req.quantity).await?;

    let description = format!("Pembayaran Order #{}", order_id);
    if let Err(e) = deduct_wallet(titipers_id, order_id, total_price, &description).await {
        let _ = release_stock(req.product_id, order_id, req.quantity).await;
        return Err(e);
    }

    let order = repo::create(
        &pool,
        titipers_id,
        jastiper_id,
        order_id,
        req,
        product_snapshot,
        unit_price,
        service_fee,
        total_price,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": "Pesanan berhasil dibuat",
            "data": order
        })),
    ))
}

// --- GET /orders/{order_id} ---
#[utoipa::path(
    get, path = "/orders/{order_id}",
    tag = "Orders",
    params(("order_id" = Uuid, Path, description = "ID unik pesanan")),
    responses(
        (status=200, description="Data pesanan ditemukan", body=Order),
        (status=401, description="Token tidak valid atau tidak ada"),
        (status=403, description="Bukan pesanan milik user ini"),
        (status=404, description="Pesanan tidak ditemukan"),
    )
)]
pub async fn get_order(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let requester_id = claims.user_id()?;

    let order = repo::find_by_id(&pool, order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    // Hanya titipers atau jastiper yang terlibat yang boleh melihat
    if order.titipers_id != requester_id && order.jastiper_id != requester_id {
        return Err(AppError::Forbidden(
            "Anda tidak memiliki akses ke pesanan ini".to_string(),
        ));
    }

    Ok(Json(
        json!({"success": true, "message": "OK", "data": order}),
    ))
}

// --- GET /orders/my/purchases ---
#[utoipa::path(
    get,
    path = "/orders/my/purchases",
    tag = "Orders",
    params(
        ("page" = Option<i64>, Query, description = "Halaman (default: 1)"),
        ("limit" = Option<i64>, Query, description = "Item per halaman (default: 20)")
    ),
    responses(
        (status = 200, description = "Berhasil", body = serde_json::Value),
        (status = 401, description = "Token tidak valid atau tidak ada"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn my_purchases(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let titipers_id = claims.user_id()?;

    let filter = Some(OrderFilter {
        titipers_id: Some(titipers_id),
        ..Default::default()
    });

    let (orders, total_count) = repo::find_all(&pool, filter, params.page, params.limit).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Riwayat belanja ditemukan",
        "data": orders,
        "pagination": {
            "total_items": total_count,
            "page": params.page.unwrap_or(1),
            "limit": params.limit.unwrap_or(20),
            "total_pages": (total_count as f64 / params.limit.unwrap_or(20) as f64).ceil() as i64
        }
    })))
}

// --- GET /orders/my/sales ---
#[utoipa::path(
    get,
    path = "/orders/my/sales",
    tag = "Orders",
    params(
        ("page" = Option<i64>, Query, description = "Halaman"),
        ("limit" = Option<i64>, Query, description = "Limit")
    ),
    responses(
        (status = 200, description = "Berhasil", body = serde_json::Value),
        (status = 401, description = "Token tidak valid atau tidak ada"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn my_sales(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let jastiper_id = claims.user_id()?;

    let filter = Some(OrderFilter {
        jastiper_id: Some(jastiper_id),
        ..Default::default()
    });

    let (orders, total_count) = repo::find_all(&pool, filter, params.page, params.limit).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Daftar pesanan masuk ditemukan",
        "data": orders,
        "pagination": {
            "total_items": total_count,
            "page": params.page.unwrap_or(1),
            "limit": params.limit.unwrap_or(20),
            "total_pages": (total_count as f64 / params.limit.unwrap_or(20) as f64).ceil() as i64
        }
    })))
}
