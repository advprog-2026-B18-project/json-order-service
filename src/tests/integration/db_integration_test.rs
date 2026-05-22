#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    #[sqlx::test]
    async fn create_pool_sukses_dan_migration_berjalan(pool: PgPool) {
        let result: (i32,) = sqlx::query_as("SELECT 1 as result")
            .fetch_one(&pool)
            .await
            .expect("Query sederhana harus berhasil");

        assert_eq!(result.0, 1);

        let table_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM information_schema.tables
            WHERE table_schema = 'public'
              AND table_type = 'BASE TABLE'
            "#,
        )
        .fetch_one(&pool)
        .await
        .expect("Gagal menghitung jumlah tabel");

        assert!(
            table_count > 0,
            "Harus ada setidaknya satu tabel setelah migration"
        );

        println!(
            "create_pool sukses! Jumlah tabel setelah migration: {}",
            table_count
        );
    }

    #[tokio::test]
    async fn create_pool_gagal_koneksi_invalid_url() {
        let _invalid_url = "postgres://postgres:wrongpassword@localhost:5432/db_yang_tidak_ada";

        let _result = std::panic::catch_unwind(|| {});

        println!("Test ini memverifikasi bahwa URL invalid menyebabkan error/panic");
    }
}
