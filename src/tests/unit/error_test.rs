use crate::error::AppError;
use axum::body::to_bytes;
use axum::response::IntoResponse;
use serde_json::Value;

async fn into_parts(err: AppError) -> (u16, Value) {
    let response = err.into_response();
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("failed to read body");
    let body: Value = serde_json::from_slice(&bytes).expect("body is not valid JSON");
    (status, body)
}

#[test]
fn display_validation() {
    assert_eq!(
        AppError::Validation("bad input".into()).to_string(),
        "Validation error: bad input"
    );
}

#[test]
fn display_unauthorized() {
    assert_eq!(
        AppError::Unauthorized("not logged in".into()).to_string(),
        "not logged in"
    );
}

#[test]
fn display_forbidden() {
    assert_eq!(
        AppError::Forbidden("no access".into()).to_string(),
        "no access"
    );
}

#[test]
fn display_not_found() {
    assert_eq!(AppError::NotFound("missing".into()).to_string(), "missing");
}

#[test]
fn display_conflict() {
    assert_eq!(
        AppError::Conflict("duplicate".into()).to_string(),
        "duplicate"
    );
}

#[test]
fn display_unprocessable_entity() {
    assert_eq!(
        AppError::UnprocessableEntity("can't process".into()).to_string(),
        "can't process"
    );
}

#[test]
fn display_invalid_status_transition() {
    let err = AppError::InvalidStatusTransition {
        current: "PENDING".into(),
        requested: "SHIPPED".into(),
        valid: vec!["PAID".into()],
    };
    assert_eq!(
        err.to_string(),
        "Invalid status transition from PENDING to SHIPPED"
    );
}

#[test]
fn display_limit_exceeded() {
    assert_eq!(
        AppError::LimitExceeded.to_string(),
        "Limit exceeded: max 1000"
    );
}

#[test]
fn display_internal() {
    assert_eq!(AppError::Internal.to_string(), "Internal error");
}

#[test]
fn from_sqlx_error() {
    let sqlx_err = sqlx::Error::RowNotFound;
    let app_err = AppError::from(sqlx_err);
    assert!(matches!(app_err, AppError::Database(_)));
    assert!(app_err.to_string().starts_with("Database error:"));
}

#[tokio::test]
async fn response_validation_is_400() {
    let (status, body) = into_parts(AppError::Validation("field required".into())).await;
    assert_eq!(status, 400);
    assert_eq!(body["success"], false);
    assert_eq!(body["message"], "field required");
    assert!(body["errors"].is_array());
    assert!(!body["errors"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn response_validation_errors_field_and_message() {
    let (_, body) = into_parts(AppError::Validation("too short".into())).await;
    let errors = body["errors"].as_array().unwrap();
    assert_eq!(errors[0]["field"], "unknown");
    assert_eq!(errors[0]["message"], "too short");
}

#[tokio::test]
async fn response_unauthorized_is_401() {
    let (status, body) = into_parts(AppError::Unauthorized("token expired".into())).await;
    assert_eq!(status, 401);
    assert_eq!(body["success"], false);
    assert_eq!(body["message"], "token expired");
}

#[tokio::test]
async fn response_forbidden_is_403() {
    let (status, body) = into_parts(AppError::Forbidden("access denied".into())).await;
    assert_eq!(status, 403);
    assert_eq!(body["success"], false);
    assert_eq!(body["message"], "access denied");
}

#[tokio::test]
async fn response_not_found_is_404() {
    let (status, body) = into_parts(AppError::NotFound("order not found".into())).await;
    assert_eq!(status, 404);
    assert_eq!(body["success"], false);
    assert_eq!(body["message"], "order not found");
}

#[tokio::test]
async fn response_conflict_is_409() {
    let (status, body) = into_parts(AppError::Conflict("email already exists".into())).await;
    assert_eq!(status, 409);
    assert_eq!(body["success"], false);
    assert_eq!(body["message"], "email already exists");
}

#[tokio::test]
async fn response_unprocessable_entity_is_422() {
    let (status, body) = into_parts(AppError::UnprocessableEntity(
        "cannot cancel shipped order".into(),
    ))
    .await;
    assert_eq!(status, 422);
    assert_eq!(body["success"], false);
    assert_eq!(body["message"], "cannot cancel shipped order");
}

#[tokio::test]
async fn response_invalid_status_transition_is_422() {
    let err = AppError::InvalidStatusTransition {
        current: "PAID".into(),
        requested: "PENDING".into(),
        valid: vec!["PURCHASED".into(), "REFUNDING".into()],
    };
    let (status, body) = into_parts(err).await;
    assert_eq!(status, 422);
    assert_eq!(body["success"], false);
    assert_eq!(body["message"], "Invalid status transition");
    assert_eq!(body["current_status"], "PAID");
    assert_eq!(body["requested_status"], "PENDING");
    let valid: Vec<&str> = body["valid_transitions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(valid.contains(&"PURCHASED"));
    assert!(valid.contains(&"REFUNDING"));
}

#[tokio::test]
async fn response_invalid_status_transition_empty_valid_list() {
    let err = AppError::InvalidStatusTransition {
        current: "COMPLETED".into(),
        requested: "PENDING".into(),
        valid: vec![],
    };
    let (status, body) = into_parts(err).await;
    assert_eq!(status, 422);
    assert!(body["valid_transitions"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn response_database_is_500() {
    let (status, body) = into_parts(AppError::Database(sqlx::Error::RowNotFound)).await;
    assert_eq!(status, 500);
    assert_eq!(body["success"], false);
}

#[tokio::test]
async fn response_limit_exceeded_is_400() {
    let (status, body) = into_parts(AppError::LimitExceeded).await;
    assert_eq!(status, 400);
    assert_eq!(body["success"], false);
    assert_eq!(body["message"], "Limit exceeded: max 1000");
}

#[tokio::test]
async fn response_internal_is_500() {
    let (status, body) = into_parts(AppError::Internal).await;
    assert_eq!(status, 500);
    assert_eq!(body["success"], false);
    assert_eq!(body["message"], "Internal error");
}

#[tokio::test]
async fn response_content_type_is_json() {
    let response = AppError::Internal.into_response();
    let ct = response
        .headers()
        .get("content-type")
        .expect("missing content-type")
        .to_str()
        .unwrap();
    assert!(ct.contains("application/json"), "content-type was: {ct}");
}

#[test]
#[allow(clippy::unnecessary_literal_unwrap)]
fn result_type_alias_ok() {
    let r: Result<i32, AppError> = Ok(42);
    assert!(r.is_ok());
    assert_eq!(r.unwrap(), 42);
}

#[test]
fn result_type_alias_err() {
    let r: Result<i32, AppError> = Err(AppError::Internal);
    assert!(r.is_err());
}

#[tokio::test]
async fn response_insufficient_balance_is_402() {
    use axum::response::IntoResponse;
    let response = AppError::InsufficientBalance.into_response();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(status, axum::http::StatusCode::PAYMENT_REQUIRED);
    assert_eq!(body["success"], false);
    assert_eq!(body["message"], "Insufficient balance");
}
