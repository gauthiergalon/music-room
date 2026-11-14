use std::time::Duration;

use sqlx::postgres::PgPoolOptions;

pub type DbPool = sqlx::PgPool;

const CONNECT_ATTEMPTS: u32 = 5;
const DEFAULT_MAX_CONNECTIONS: u32 = 10;

/// Connects to PostgreSQL, retrying while the database container starts.
pub async fn create_pool(database_url: &str) -> Result<DbPool, sqlx::Error> {
    let max_connections = std::env::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_MAX_CONNECTIONS);

    let mut attempt = 1;
    loop {
        match PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await
        {
            Ok(pool) => return Ok(pool),
            Err(e) if attempt < CONNECT_ATTEMPTS => {
                tracing::warn!(
                    "PostgreSQL connection attempt {}/{} failed: {}",
                    attempt,
                    CONNECT_ATTEMPTS,
                    e
                );
                attempt += 1;
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(e) => return Err(e),
        }
    }
}

/// Applies the SQL files of `migrations/` that this database has not seen yet.
pub async fn run_migrations(pool: &DbPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
