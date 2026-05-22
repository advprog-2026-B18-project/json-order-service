use crate::repositories::adapters::idempotency_adapt::PgIdempotencyRepository;
use crate::repositories::idempotency_repository::IdempotencyRepository;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

async fn insert_order(pool: &PgPool, order_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO "order" (
            order_id, titipers_id, jastiper_id, product_id,
            product_snapshot, quantity, unit_price, service_fee,
            total_price, status, shipping_address
        )
        VALUES ($1, $2, $3, $4, $5, 1, 10, 1, 11, 'RESERVING', $6)
        "#,
    )
    .bind(order_id)
    .bind(Uuid::new_v4())
    .bind(Uuid::new_v4())
    .bind(Uuid::new_v4())
    .bind(json!({ "name": "test" }))
    .bind(json!({ "street": "test" }))
    .execute(pool)
    .await
    .expect("failed to insert order");
}

// === Happy Path ===
#[sqlx::test(migrations = "./migrations")]
async fn test_pg_idempotency_repository_new_unprocessed_key_returns_false(pool: PgPool) {
    let repo = PgIdempotencyRepository::new(pool);
    let key = Uuid::new_v4();

    let result = repo.is_processed(key).await;

    assert!(matches!(result, Ok(false)));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_pg_idempotency_repository_mark_processed_then_is_processed_returns_true(
    pool: PgPool,
) {
    let repo = PgIdempotencyRepository::new(pool);
    let key = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    insert_order(&repo.pool, order_id).await;

    let mark_result = repo.mark_processed(key, order_id).await;
    let processed_result = repo.is_processed(key).await;

    assert!(mark_result.is_ok());
    assert!(matches!(processed_result, Ok(true)));
}

// === Edge Cases ===
#[sqlx::test(migrations = "./migrations")]
async fn test_pg_idempotency_repository_duplicate_mark_processed_returns_ok(pool: PgPool) {
    let repo = PgIdempotencyRepository::new(pool);
    let key = Uuid::new_v4();
    let order_id = Uuid::new_v4();
    insert_order(&repo.pool, order_id).await;

    let first_result = repo.mark_processed(key, order_id).await;
    let duplicate_result = repo.mark_processed(key, Uuid::new_v4()).await;

    assert!(first_result.is_ok());
    assert!(duplicate_result.is_ok());
}
