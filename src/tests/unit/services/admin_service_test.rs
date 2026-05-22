use std::sync::Arc;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::filter_pagination::{OrderFilter, PaginationParams};
use crate::models::role::Role;
use crate::repositories::order_repository::MockOrderRepository;
use crate::services::admin::{force_cancel, get_all, get_order};

// ──────────────────────────────────────────────────────────────
// get_all
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_admin_get_all_db_error_returns_error() {
    // Arrange
    let mut order_repo = MockOrderRepository::new();
    order_repo
        .expect_find_all()
        .returning(|_, _| Err(AppError::Internal));

    let filter = OrderFilter::default();
    let pagination = PaginationParams {
        page: None,
        limit: None,
        sort_by: None,
        order: None,
    };

    // Act
    let result = get_all(Arc::new(order_repo), &filter, &pagination, &Role::Admin).await;

    // Assert
    assert!(matches!(result, Err(AppError::Internal)));
}

#[tokio::test]
async fn test_admin_get_all_non_admin_forbidden() {
    // Arrange
    let order_repo = MockOrderRepository::new();
    let filter = OrderFilter::default();
    let pagination = PaginationParams {
        page: None,
        limit: None,
        sort_by: None,
        order: None,
    };

    // Act
    let result = get_all(Arc::new(order_repo), &filter, &pagination, &Role::Titipers).await;

    // Assert
    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn test_admin_get_all_success_returns_orders() {
    // Arrange
    let mut order_repo = MockOrderRepository::new();
    order_repo
        .expect_find_all()
        .returning(|_, _| Ok((vec![], 0)));

    let filter = OrderFilter::default();
    let pagination = PaginationParams {
        page: None,
        limit: None,
        sort_by: None,
        order: None,
    };

    // Act
    let result = get_all(Arc::new(order_repo), &filter, &pagination, &Role::Admin).await;

    // Assert
    assert!(result.is_ok());
}

// ──────────────────────────────────────────────────────────────
// get_order
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_admin_get_order_db_error_returns_error() {
    // Arrange
    let mut order_repo = MockOrderRepository::new();
    order_repo
        .expect_find_by_id()
        .returning(|_| Err(AppError::Internal));

    // Act
    let result = get_order(Arc::new(order_repo), Uuid::new_v4(), &Role::Admin).await;

    // Assert
    assert!(matches!(result, Err(AppError::Internal)));
}

#[tokio::test]
async fn test_admin_get_order_non_admin_forbidden() {
    // Arrange
    let order_repo = MockOrderRepository::new();

    // Act
    let result = get_order(Arc::new(order_repo), Uuid::new_v4(), &Role::Jastiper).await;

    // Assert
    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn test_admin_get_order_not_found_returns_not_found() {
    // Arrange
    let mut order_repo = MockOrderRepository::new();
    order_repo.expect_find_by_id().returning(|_| Ok(None));

    // Act
    let result = get_order(Arc::new(order_repo), Uuid::new_v4(), &Role::Admin).await;

    // Assert
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

// ──────────────────────────────────────────────────────────────
// force_cancel
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_admin_force_cancel_non_admin_forbidden() {
    // Arrange
    let order_repo = MockOrderRepository::new();
    let inventory = crate::services::inventory_client::MockInventoryClient::new();
    let wallet = crate::services::wallet_client::MockWalletClient::new();

    // Act
    let result = force_cancel(
        Arc::new(order_repo),
        Arc::new(inventory),
        Arc::new(wallet),
        Uuid::new_v4(),
        Uuid::new_v4(),
        &Role::Titipers,
        crate::models::order::CancelRequest {
            cancellation_reason: "test".to_string(),
        },
    )
    .await;

    // Assert
    assert!(matches!(result, Err(AppError::Forbidden(_))));
}
