use crate::error::AppError;
use crate::middleware::auth::JwtClaims;
use crate::models::order::{
    CancelRequest, CancelledBy, CreateOrderRequest, OrderFilter, OrderStatus,
    PaginationParams, UpdateStatusRequest,
};
use crate::repositories::order as repo;
use axum::Json;
use axum::extract::{Path, Query, State};
use reqwest::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

fn inventory_url() -> String {
    std::env::var("INVENTORY_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:8081".to_string())
}

fn wallet_url() -> String {
    std::env::var("WALLET_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:8082".to_string())
}

fn service_key() -> String {
    std::env::var("INTERNAL_SERVICE_KEY")
        .expect("INTERNAL_SERVICE_KEY harus diset di .env")
}

async fn internal_post(url: &str, body: serde_json::Value) -> Result<u16, AppError> {
    let status = reqwest::Client::new()
        .post(url)
        .header("X-Service-Key", service_key())
        .json(&body)
        .send()
        .await
        .map_err(|_| AppError::Internal)?
        .status()
        .as_u16();
    Ok(status)
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
    let url = format!(
        "{}/internal/products/{}/stock/{}",
        inventory_url(),
        product_id,
        suffix
    );
    match internal_post(&url, json!({ "order_id": order_id, "quantity": quantity })).await? {
        200 => Ok(()),
        404 => Err(AppError::NotFound("Produk tidak ditemukan".to_string())),
        409 => Err(AppError::Conflict("Stok tidak mencukupi".to_string())),
        422 => Err(AppError::UnprocessableEntity(
            "Produk tidak dalam status ACTIVE".to_string(),
        )),
        _ => Err(AppError::Internal),
    }
}

pub(crate) async fn reserve_stock(
    product_id: Uuid,
    order_id: Uuid,
    quantity: i32,
) -> Result<(), AppError> {
    manage_stock(StockAction::Reserve, product_id, order_id, quantity).await
}

pub(crate) async fn release_stock(
    product_id: Uuid,
    order_id: Uuid,
    quantity: i32,
) -> Result<(), AppError> {
    manage_stock(StockAction::Release, product_id, order_id, quantity).await
}

enum WalletAction {
    Deduct,
    Refund,
}

async fn manage_wallet(
    action: WalletAction,
    user_id: Uuid,
    order_id: Uuid,
    amount: i64,
    description: &str,
) -> Result<(), AppError> {
    let endpoint = match action {
        WalletAction::Deduct => "deduct",
        WalletAction::Refund => "refund",
    };
    let url = format!("{}/internal/wallets/{}", wallet_url(), endpoint);
    let body = json!({
        "user_id":     user_id,
        "order_id":    order_id,
        "amount":      amount,
        "description": description,
    });

    match (action, internal_post(&url, body).await?) {
        (WalletAction::Deduct, 200) => Ok(()),
        (WalletAction::Deduct, 404) => Err(AppError::NotFound("User tidak ditemukan".to_string())),
        (WalletAction::Deduct, 409) => Ok(()), // idempotent
        (WalletAction::Deduct, 422) => Err(AppError::UnprocessableEntity(
            "Saldo tidak mencukupi".to_string(),
        )),
        (WalletAction::Deduct, _) => Err(AppError::Internal),
        (WalletAction::Refund, 200) | (WalletAction::Refund, 409) => Ok(()), // 409 = sudah direfund
        (WalletAction::Refund, _) => Err(AppError::Internal),
    }
}

pub(crate) async fn deduct_wallet(
    user_id: Uuid,
    order_id: Uuid,
    amount: i64,
    description: &str,
) -> Result<(), AppError> {
    manage_wallet(WalletAction::Deduct, user_id, order_id, amount, description).await
}

pub(crate) async fn refund_wallet(
    user_id: Uuid,
    order_id: Uuid,
    amount: i64,
    description: &str,
) -> Result<(), AppError> {
    manage_wallet(WalletAction::Refund, user_id, order_id, amount, description).await
}


pub(crate) async fn fetch_product(product_id: Uuid) -> Result<serde_json::Value, AppError> {
    let url = format!(
        "{}/products/{}",
        std::env::var("INVENTORY_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8083".to_string()),
        product_id
    );
    let response = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|_| AppError::Internal)?;

    match response.status().as_u16() {
        200 => {
            let body: serde_json::Value = response.json().await.map_err(|_| AppError::Internal)?;
            Ok(body["data"].clone())
        }
        404 => Err(AppError::NotFound("Produk tidak ditemukan".to_string())),
        422 => Err(AppError::UnprocessableEntity("Produk tidak aktif".to_string())),
        _ => Err(AppError::Internal),
    }
}


fn paginated_response(
    message: &str,
    orders: impl serde::Serialize,
    total_count: i64,
    page: Option<i64>,
    limit: Option<i64>,
) -> serde_json::Value {
    let page  = page.unwrap_or(1);
    let limit = limit.unwrap_or(20);
    json!({
        "success": true,
        "message": message,
        "data":    orders,
        "pagination": {
            "total_items": total_count,
            "page":        page,
            "limit":       limit,
            "total_pages": (total_count as f64 / limit as f64).ceil() as i64,
        }
    })
}


async fn fetch_order_with_access_check(
    pool: &PgPool,
    order_id: Uuid,
    requester_id: Uuid,
) -> Result<crate::models::order::Order, AppError> {
    let order = repo::find_by_id(pool, order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    if order.titipers_id != requester_id && order.jastiper_id != requester_id {
        return Err(AppError::Forbidden(
            "Anda tidak memiliki akses ke pesanan ini".to_string(),
        ));
    }
    Ok(order)
}


/// POST /orders
#[utoipa::path(
    post, path = "/orders",
    tag = "Orders",
    request_body = CreateOrderRequest,
    responses(
        (status=201, description="Pesanan berhasil dibuat",        body=Order),
        (status=400, description="Data tidak valid"),
        (status=401, description="Token tidak valid atau tidak ada"),
        (status=403, description="Jastiper mencoba beli produk sendiri"),
        (status=409, description="Stok tidak mencukupi"),
        (status=422, description="Saldo tidak cukup / produk tidak aktif"),
    )
)]
pub async fn checkout(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Json(req): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let titipers_id = claims.user_id()?;
    let order_id   = Uuid::new_v4();

    let product = fetch_product(req.product_id).await?;
    let jastiper_id: Uuid =
        serde_json::from_value(product["jastiper_id"].clone()).map_err(|_| AppError::Internal)?;

    if titipers_id == jastiper_id {
        return Err(AppError::Forbidden(
            "Jastiper tidak dapat membeli produk milik sendiri".to_string(),
        ));
    }

    let unit_price  = product["price"].as_i64().unwrap_or(0);
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

    // 1. Reserve stok
    reserve_stock(req.product_id, order_id, req.quantity).await?;

    // 2. Deduct wallet — rollback stok jika gagal
    let description = format!("Pembayaran Order #{}", order_id);
    if let Err(e) = deduct_wallet(titipers_id, order_id, total_price, &description).await {
        let _ = release_stock(req.product_id, order_id, req.quantity).await;
        return Err(e);
    }

    // 3. Simpan ke DB — rollback stok + refund wallet jika gagal
    let product_id = req.product_id;
    let quantity   = req.quantity;
    match repo::create(
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
        .await
    {
        Ok(order) => Ok((
            StatusCode::CREATED,
            Json(json!({ "success": true, "message": "Pesanan berhasil dibuat", "data": order })),
        )),
        Err(e) => {
            let _ = release_stock(product_id, order_id, quantity).await;
            let refund_desc = format!("Refund Order #{} - gagal menyimpan pesanan", order_id);
            let _ = refund_wallet(titipers_id, order_id, total_price, &refund_desc).await;
            Err(e)
        }
    }
}

/// GET /orders/{order_id}
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
    let order = fetch_order_with_access_check(&pool, order_id, requester_id).await?;
    Ok(Json(json!({ "success": true, "message": "OK", "data": order })))
}

/// GET /orders/{order_id}/history
#[utoipa::path(
    get, path = "/orders/{order_id}/history",
    tag = "Orders",
    params(("order_id" = Uuid, Path, description = "ID unik pesanan")),
    responses(
        (status=200, description="Riwayat status pesanan"),
        (status=401, description="Token tidak valid atau tidak ada"),
        (status=403, description="Bukan pesanan milik user ini"),
        (status=404, description="Pesanan tidak ditemukan"),
    )
)]
pub async fn get_order_history(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let requester_id = claims.user_id()?;
    fetch_order_with_access_check(&pool, order_id, requester_id).await?;

    let history = repo::get_status_history(&pool, order_id).await?;
    Ok(Json(json!({ "success": true, "message": "Riwayat status ditemukan", "data": history })))
}

/// PATCH /orders/{order_id}/status
#[utoipa::path(
    patch, path = "/orders/{order_id}/status",
    tag = "Orders",
    params(("order_id" = Uuid, Path, description = "ID unik pesanan")),
    request_body = UpdateStatusRequest,
    responses(
        (status=200, description="Status berhasil diperbarui",     body=Order),
        (status=400, description="Data tidak valid"),
        (status=401, description="Token tidak valid atau tidak ada"),
        (status=403, description="Tidak punya izin mengubah status ini"),
        (status=404, description="Pesanan tidak ditemukan"),
        (status=422, description="Transisi status tidak valid"),
    )
)]
pub async fn update_status(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let requester_id = claims.user_id()?;
    let role = &claims.role;

    let order = repo::find_by_id(&pool, order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    match (&req.status, role.as_str()) {
        // Jastiper: PAID → PURCHASED, PURCHASED → SHIPPED
        (OrderStatus::Purchased, "JASTIPER") | (OrderStatus::Shipped, "JASTIPER") => {
            if order.jastiper_id != requester_id {
                return Err(AppError::Forbidden(
                    "Hanya jastiper pemilik produk yang dapat mengubah status ini".to_string(),
                ));
            }
        }
        // Titipers: SHIPPED → COMPLETED
        (OrderStatus::Completed, "TITIPERS") => {
            if order.titipers_id != requester_id {
                return Err(AppError::Forbidden(
                    "Hanya titipers pemilik order yang dapat mengkonfirmasi penerimaan".to_string(),
                ));
            }
        }
        // Admin boleh semua transisi
        (_, "ADMIN") => {}
        _ => {
            return Err(AppError::Forbidden(
                "Role Anda tidak memiliki izin untuk transisi status ini".to_string(),
            ));
        }
    }

    let updated = repo::update_status(
        &pool,
        order_id,
        &req.status,
        &requester_id.to_string(),
        &role.to_uppercase(),
        req.notes.as_deref(),
        req.tracking_number.as_deref(),
        req.courier.as_deref(),
    )
        .await?;

    Ok(Json(json!({
        "success": true,
        "message": "Status pesanan berhasil diperbarui",
        "data": updated
    })))
}

/// POST /orders/{order_id}/cancel
#[utoipa::path(
    post, path = "/orders/{order_id}/cancel",
    tag = "Orders",
    params(("order_id" = Uuid, Path, description = "ID unik pesanan")),
    request_body = CancelRequest,
    responses(
        (status=200, description="Pesanan berhasil dibatalkan",    body=Order),
        (status=400, description="Data tidak valid"),
        (status=401, description="Token tidak valid atau tidak ada"),
        (status=403, description="Tidak punya izin membatalkan pesanan ini"),
        (status=404, description="Pesanan tidak ditemukan"),
        (status=422, description="Pesanan tidak dapat dibatalkan pada status ini"),
    )
)]
pub async fn cancel_order(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Path(order_id): Path<Uuid>,
    Json(req): Json<CancelRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let requester_id = claims.user_id()?;
    let role = &claims.role;

    let order = repo::find_by_id(&pool, order_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Pesanan tidak ditemukan".to_string()))?;

    let (cancelled_by, actor_role) = match role.as_str() {
        "TITIPERS" => {
            if order.titipers_id != requester_id {
                return Err(AppError::Forbidden(
                    "Hanya titipers pemilik order yang dapat membatalkan".to_string(),
                ));
            }
            if order.status != OrderStatus::Paid {
                return Err(AppError::UnprocessableEntity(
                    "Titipers hanya dapat membatalkan pesanan dengan status PAID".to_string(),
                ));
            }
            (CancelledBy::Jastiper, "TITIPERS")
        }
        "JASTIPER" => {
            if order.jastiper_id != requester_id {
                return Err(AppError::Forbidden(
                    "Hanya jastiper pemilik produk yang dapat membatalkan".to_string(),
                ));
            }
            (CancelledBy::Jastiper, "JASTIPER")
        }
        "ADMIN" => (CancelledBy::Admin, "ADMIN"),
        _ => {
            return Err(AppError::Forbidden(
                "Role Anda tidak memiliki izin membatalkan pesanan".to_string(),
            ));
        }
    };

    let updated = repo::cancel_order(
        &pool,
        order_id,
        &req.cancellation_reason,
        &cancelled_by,
        &requester_id.to_string(),
        actor_role,
        req.notes.as_deref(),
    )
        .await?;

    // Kompensasi: kembalikan stok
    let product_id: Uuid =
        serde_json::from_value(updated.product_snapshot["product_id"].clone())
            .unwrap_or(updated.product_id);
    let _ = release_stock(product_id, order_id, updated.quantity).await;

    // Kompensasi: refund saldo ke titipers
    let refund_desc = format!("Refund Order #{} - pesanan dibatalkan", order_id);
    let _ = refund_wallet(updated.titipers_id, order_id, updated.total_price, &refund_desc).await;

    Ok(Json(json!({
        "success": true,
        "message": "Pesanan berhasil dibatalkan",
        "data": updated
    })))
}

/// GET /orders/my/purchases
#[utoipa::path(
    get, path = "/orders/my/purchases",
    tag = "Orders",
    params(
        ("page"  = Option<i64>, Query, description = "Halaman (default: 1)"),
        ("limit" = Option<i64>, Query, description = "Item per halaman (default: 20)")
    ),
    responses(
        (status=200, description="Berhasil",                       body=serde_json::Value),
        (status=401, description="Token tidak valid atau tidak ada"),
        (status=500, description="Internal server error")
    )
)]
pub async fn my_purchases(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let titipers_id = claims.user_id()?;
    let filter = Some(OrderFilter { titipers_id: Some(titipers_id), ..Default::default() });
    let (orders, total_count) = repo::find_all(&pool, filter, params.page, params.limit).await?;
    Ok(Json(paginated_response(
        "Riwayat belanja ditemukan",
        orders, total_count, params.page, params.limit,
    )))
}

/// GET /orders/my/sales
#[utoipa::path(
    get, path = "/orders/my/sales",
    tag = "Orders",
    params(
        ("page"  = Option<i64>, Query, description = "Halaman"),
        ("limit" = Option<i64>, Query, description = "Limit")
    ),
    responses(
        (status=200, description="Berhasil",                       body=serde_json::Value),
        (status=401, description="Token tidak valid atau tidak ada"),
        (status=500, description="Internal server error")
    )
)]
pub async fn my_sales(
    State(pool): State<Arc<PgPool>>,
    claims: JwtClaims,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let jastiper_id = claims.user_id()?;
    let filter = Some(OrderFilter { jastiper_id: Some(jastiper_id), ..Default::default() });
    let (orders, total_count) = repo::find_all(&pool, filter, params.page, params.limit).await?;
    Ok(Json(paginated_response(
        "Daftar pesanan masuk ditemukan",
        orders, total_count, params.page, params.limit,
    )))
}