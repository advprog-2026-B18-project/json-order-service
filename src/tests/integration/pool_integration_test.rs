#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    #[sqlx::test(migrations = "./migrations")]
    async fn create_pool_berhasil_connect_dan_migrate(pool: PgPool) {
        assert!(!pool.is_closed());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn pool_bisa_execute_query(pool: PgPool) {
        let result = sqlx::query("SELECT 1 as val").fetch_one(&pool).await;

        assert!(result.is_ok());
    }
}
