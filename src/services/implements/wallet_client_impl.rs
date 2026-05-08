use crate::error::AppError;
use crate::services::http_client::{internal_get, internal_post};
use crate::services::wallet_client::{DeductResponse, EarningsResponse, RefundResponse};
use serde_json::json;
use tracing::{debug, error, warn};
use uuid::Uuid;

fn wallet_url() -> String {
    let url =
        std::env::var("WALLET_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    debug!("🌐 [wallet] using URL: {}", url);
    url
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
) -> Result<String, AppError> {
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

    let (status, body) = internal_post(&url, body).await?;

    match (action, status) {
        (WalletAction::Deduct, 200) => {
            let transaction_id = body["transaction_id"].as_str().unwrap_or("").to_string();
            debug!(
                "✅ [wallet] deduct berhasil user_id={} amount={}",
                user_id, amount
            );
            Ok(transaction_id)
        }
        (WalletAction::Deduct, 404) => {
            warn!("⚠️ [wallet] user tidak ditemukan user_id={}", user_id);
            Err(AppError::NotFound("User tidak ditemukan".to_string()))
        }
        (WalletAction::Deduct, 409) => {
            let transaction_id = body["transaction_id"].as_str().unwrap_or("").to_string();
            debug!("ℹ️ [wallet] deduct idempotent order_id={}", order_id);
            Ok(transaction_id)
        }
        (WalletAction::Deduct, 422) => {
            warn!(
                "⚠️ [wallet] saldo tidak mencukupi user_id={} amount={}",
                user_id, amount
            );
            Err(AppError::UnprocessableEntity(
                "Saldo tidak mencukupi".to_string(),
            ))
        }
        (WalletAction::Deduct, code) => {
            error!(
                "❌ [wallet] deduct unexpected status={} user_id={}",
                code, user_id
            );
            Err(AppError::Internal)
        }
        (WalletAction::Refund, 200) => {
            let transaction_id = body["transaction_id"].as_str().unwrap_or("").to_string();
            debug!(
                "✅ [wallet] refund berhasil user_id={} amount={}",
                user_id, amount
            );
            Ok(transaction_id)
        }
        (WalletAction::Refund, 409) => {
            let transaction_id = body["transaction_id"].as_str().unwrap_or("").to_string();
            debug!("ℹ️ [wallet] refund idempotent order_id={}", order_id);
            Ok(transaction_id)
        }
        (WalletAction::Refund, code) => {
            error!(
                "❌ [wallet] refund unexpected status={} user_id={}",
                code, user_id
            );
            Err(AppError::Internal)
        }
    }
}

pub(crate) async fn deduct_wallet(
    user_id: Uuid,
    order_id: Uuid,
    amount: i64,
    description: &str,
) -> Result<DeductResponse, AppError> {
    debug!(
        "💳 [wallet] deduct_wallet user_id={} order_id={} amount={}",
        user_id, order_id, amount
    );
    let transaction_id =
        manage_wallet(WalletAction::Deduct, user_id, order_id, amount, description).await?;
    Ok(DeductResponse { transaction_id })
}

pub(crate) async fn refund_wallet(
    user_id: Uuid,
    order_id: Uuid,
    amount: i64,
    description: &str,
) -> Result<RefundResponse, AppError> {
    debug!(
        "💳 [wallet] refund_wallet user_id={} order_id={} amount={}",
        user_id, order_id, amount
    );
    let transaction_id =
        manage_wallet(WalletAction::Refund, user_id, order_id, amount, description).await?;
    Ok(RefundResponse { transaction_id })
}

pub(crate) async fn check_wallet(user_id: Uuid, req_amount: i64) -> Result<(), AppError> {
    debug!("💳 [wallet] deduct_wallet user_id={}", user_id);

    let url = format!("{}/internal/wallets/balance-check", wallet_url());
    let body = json!({
        "user_id":     user_id,
        "required_amount": req_amount,
    });

    let (status, body) = internal_get(&url, body).await?;

    match status {
        200 => {
            let is_sufficient = body["is_sufficient"].as_bool().unwrap_or(false);
            if is_sufficient {
                debug!(
                    "✅ [wallet] check wallet berhasil untuk user_id={} dengan amount={}",
                    user_id, req_amount
                );
                Ok(())
            } else {
                warn!(
                    "⚠️ [wallet] saldo tidak mencukupi user_id={} amount={}",
                    user_id, req_amount
                );
                Err(AppError::UnprocessableEntity(
                    "Saldo tidak mencukupi".to_string(),
                ))
            }
        }
        404 => {
            debug!(
                "✅ [wallet] check wallet gagal untuk user_id={} tidak ditemukan",
                user_id
            );
            Ok(())
        }
        _ => {
            error!(
                "❌ [wallet] check wallet unexpected status={} user_id={}",
                status, user_id
            );
            Err(AppError::Internal)
        }
    }
}

pub(crate) async fn earnings_wallet(
    jastiper_id: Uuid,
    order_id: Uuid,
    description: &str,
) -> Result<EarningsResponse, AppError> {
    let url = format!("{}/internal/wallets/earnings", wallet_url());
    let body = json!({
        "jastiper_id": jastiper_id,
        "order_id":    order_id,
        "description": description,
    });

    let (status, body) = internal_post(&url, body).await?;

    match status {
        200 => match body["status"].as_str() {
            Some("SUCCESS") => {
                let transaction_id = body["transaction_id"].as_str().unwrap_or("").to_string();
                debug!(
                    "✅ [wallet] earnings_wallet berhasil jastiper_id={}",
                    jastiper_id
                );
                Ok(EarningsResponse { transaction_id })
            }
            _ => {
                warn!(
                    "❌ [wallet] earnings_wallet GAGAL jastiper_id={}",
                    jastiper_id
                );
                Err(AppError::UnprocessableEntity(
                    "Pendapatan gagal dikreditkan".to_string(),
                ))
            }
        },
        409 => {
            warn!("⚠️ [wallet] earnings sudah diproses order_id={}", order_id);
            Err(AppError::Conflict(
                "Pendapatan untuk order ini sudah diproses".to_string(),
            ))
        }
        404 => {
            let message = body["message"].as_str().unwrap_or("Not found");
            Err(AppError::NotFound(message.to_string()))
        }
        _ => {
            error!("❌ [wallet] earnings unexpected status={}", status);
            Err(AppError::Internal)
        }
    }
}

pub(crate) async fn reverse_earnings(
    jastiper_id: Uuid,
    order_id: Uuid,
    transaction_id: &str,
    description: &str,
) -> Result<(), AppError> {
    debug!(
        "↩️  [wallet] reverse_earnings jastiper_id={} order_id={} txn_id={}",
        jastiper_id, order_id, transaction_id
    );

    let url = format!("{}/internal/wallets/earnings/reverse", wallet_url());
    let body = json!({
        "jastiper_id":    jastiper_id,
        "order_id":       order_id,
        "transaction_id": transaction_id,
        "description":    description,
    });

    let (status, _body) = internal_post(&url, body).await?;

    match status {
        200 => {
            debug!(
                "✅ [wallet] reverse_earnings berhasil jastiper_id={} txn_id={}",
                jastiper_id, transaction_id
            );
            Ok(())
        }
        404 => {
            warn!(
                "⚠️ [wallet] earnings tidak ditemukan untuk direvert txn_id={}",
                transaction_id
            );
            Err(AppError::NotFound(
                "Transaksi earnings tidak ditemukan".to_string(),
            ))
        }
        409 => {
            debug!(
                "ℹ️ [wallet] reverse_earnings sudah diproses sebelumnya txn_id={} (idempotent)",
                transaction_id
            );
            Ok(())
        }
        422 => {
            warn!(
                "⚠️ [wallet] earnings tidak bisa direvert (mungkin sudah dicairkan) txn_id={}",
                transaction_id
            );
            Err(AppError::UnprocessableEntity(
                "Earnings tidak dapat direvert".to_string(),
            ))
        }
        code => {
            error!(
                "❌ [wallet] reverse_earnings unexpected status={} txn_id={}",
                code, transaction_id
            );
            Err(AppError::Internal)
        }
    }
}
