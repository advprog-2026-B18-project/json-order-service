// tests/integration_order.rs
// Integrasi test — membutuhkan DATABASE_URL di environment
// Jalankan: DATABASE_URL=... cargo test --test integration_order

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

/// Setup: jalankan migrasi di pool test
async fn setup_db(pool: &PgPool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("Migration gagal");
}

// ─────────────────────────────────────────────────────────────────────────────
// Order Integration Tests
// ─────────────────────────────────────────────────────────────────────────────

#[sqlx::test]
async fn test_insert_and_fetch_order(pool: PgPool) {
    setup_db(&pool).await;

    let order_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO orders (
            id, titipers_id, jastiper_id, product_id,
            quantity, shipping_address, total_price,
            status, discount_amount, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4,
            $5, $6, $7,
            'PENDING', 0, $8, $8
        )
        "#,
        order_id,
        titipers_id,
        jastiper_id,
        product_id,
        2i32,
        "Jl. Sudirman No. 1, Jakarta",
        150_000i64,
        Utc::now()
    )
        .execute(&pool)
        .await
        .expect("Insert order gagal");

    let row = sqlx::query!(
        "SELECT id, status FROM orders WHERE id = $1",
        order_id
    )
        .fetch_one(&pool)
        .await
        .expect("Fetch order gagal");

    assert_eq!(row.id, order_id);
    assert_eq!(row.status, "PENDING");
}

#[sqlx::test]
async fn test_update_order_status(pool: PgPool) {
    setup_db(&pool).await;

    let order_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO orders (
            id, titipers_id, jastiper_id, product_id,
            quantity, shipping_address, total_price,
            status, discount_amount, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4,
            1, 'Test Address', 100000,
            'PENDING', 0, NOW(), NOW()
        )
        "#,
        order_id,
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4()
    )
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query!(
        "UPDATE orders SET status = 'PAID', updated_at = NOW() WHERE id = $1",
        order_id
    )
        .execute(&pool)
        .await
        .expect("Update status gagal");

    let row = sqlx::query!("SELECT status FROM orders WHERE id = $1", order_id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(row.status, "PAID");
}

#[sqlx::test]
async fn test_fetch_orders_by_titipers(pool: PgPool) {
    setup_db(&pool).await;

    let titipers_id = Uuid::new_v4();

    // Insert 2 orders untuk titipers yang sama
    for _ in 0..2 {
        sqlx::query!(
            r#"
            INSERT INTO orders (
                id, titipers_id, jastiper_id, product_id,
                quantity, shipping_address, total_price,
                status, discount_amount, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4,
                1, 'Alamat Test', 50000,
                'PENDING', 0, NOW(), NOW()
            )
            "#,
            Uuid::new_v4(),
            titipers_id,
            Uuid::new_v4(),
            Uuid::new_v4()
        )
            .execute(&pool)
            .await
            .unwrap();
    }

    let rows = sqlx::query!(
        "SELECT id FROM orders WHERE titipers_id = $1",
        titipers_id
    )
        .fetch_all(&pool)
        .await
        .unwrap();

    assert_eq!(rows.len(), 2);
}

// ─────────────────────────────────────────────────────────────────────────────
// Rating Integration Tests
// ─────────────────────────────────────────────────────────────────────────────

#[sqlx::test]
async fn test_insert_rating(pool: PgPool) {
    setup_db(&pool).await;

    // Buat order dulu (FK constraint)
    let order_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();
    let jastiper_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO orders (
            id, titipers_id, jastiper_id, product_id,
            quantity, shipping_address, total_price,
            status, discount_amount, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4,
            1, 'Test', 100000, 'COMPLETED', 0, NOW(), NOW()
        )
        "#,
        order_id,
        titipers_id,
        jastiper_id,
        product_id
    )
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query!(
        r#"
        INSERT INTO ratings (id, order_id, titipers_id, jastiper_id, product_id,
                             jastiper_rating, product_rating, review)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        Uuid::new_v4(),
        order_id,
        titipers_id,
        jastiper_id,
        product_id,
        5i16,
        4i16,
        "Sangat memuaskan!"
    )
        .execute(&pool)
        .await
        .expect("Insert rating gagal");

    let row = sqlx::query!(
        "SELECT jastiper_rating FROM ratings WHERE order_id = $1",
        order_id
    )
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(row.jastiper_rating, Some(5));
}

// ─────────────────────────────────────────────────────────────────────────────
// WarQueue Integration Tests
// ─────────────────────────────────────────────────────────────────────────────

#[sqlx::test]
async fn test_insert_and_fetch_war_queue(pool: PgPool) {
    setup_db(&pool).await;

    let entry_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let titipers_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO war_queue (id, product_id, titipers_id, quantity, status)
        VALUES ($1, $2, $3, $4, 'Waiting')
        "#,
        entry_id,
        product_id,
        titipers_id,
        2i32
    )
        .execute(&pool)
        .await
        .expect("Insert war_queue gagal");

    let row = sqlx::query!(
        "SELECT status, quantity FROM war_queue WHERE id = $1",
        entry_id
    )
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(row.status, "Waiting");
    assert_eq!(row.quantity, 2);
}

#[sqlx::test]
async fn test_war_queue_position_by_joined_at(pool: PgPool) {
    setup_db(&pool).await;

    let product_id = Uuid::new_v4();

    // Insert 3 entri secara berurutan
    for i in 0..3 {
        sqlx::query!(
            r#"
            INSERT INTO war_queue (id, product_id, titipers_id, quantity, status, joined_at)
            VALUES ($1, $2, $3, 1, 'Waiting', NOW() + ($4 * interval '1 second'))
            "#,
            Uuid::new_v4(),
            product_id,
            Uuid::new_v4(),
            i as f64
        )
            .execute(&pool)
            .await
            .unwrap();
    }

    let rows = sqlx::query!(
        "SELECT id FROM war_queue WHERE product_id = $1 ORDER BY joined_at ASC",
        product_id
    )
        .fetch_all(&pool)
        .await
        .unwrap();

    assert_eq!(rows.len(), 3, "Harus ada 3 entri di antrian");
}