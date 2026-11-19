use std::time::Duration;

use sqlx::postgres::PgPoolOptions;

pub type DbPool = sqlx::PgPool;

/// The backend and PostgreSQL may start at the same time: keep trying for
/// this long, often enough to connect as soon as the database is up. A new
/// database takes the longest (its first initialisation).
const CONNECT_DEADLINE: Duration = Duration::from_secs(30);
const CONNECT_RETRY_DELAY: Duration = Duration::from_millis(500);
const DEFAULT_MAX_CONNECTIONS: u32 = 10;

/// Connects to PostgreSQL, retrying while the database starts.
pub async fn create_pool(database_url: &str) -> Result<DbPool, sqlx::Error> {
    let max_connections = std::env::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_MAX_CONNECTIONS);

    let deadline = tokio::time::Instant::now() + CONNECT_DEADLINE;
    let mut attempts = 0;
    loop {
        attempts += 1;
        match PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await
        {
            Ok(pool) => return Ok(pool),
            Err(e) if tokio::time::Instant::now() + CONNECT_RETRY_DELAY < deadline => {
                // One line, not one per attempt: a slow start is expected.
                if attempts == 1 {
                    tracing::info!("Waiting for PostgreSQL ({e})");
                }
                tokio::time::sleep(CONNECT_RETRY_DELAY).await;
            }
            Err(e) => {
                tracing::warn!("PostgreSQL still unreachable after {attempts} attempts");
                return Err(e);
            }
        }
    }
}

/// Applies the SQL files of `migrations/` that this database has not seen yet.
pub async fn run_migrations(pool: &DbPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
