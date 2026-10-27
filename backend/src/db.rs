use std::time::Duration;
use sqlx::postgres::PgPoolOptions;

pub type DbPool = sqlx::PgPool;

pub async fn create_pool(database_url: &str) -> DbPool {
    let mut last_err = String::new();
    for i in 1..=5 {
        match PgPoolOptions::new().max_connections(5).connect(database_url).await {
            Ok(pool) => return pool,
            Err(e) => {
                last_err = e.to_string();
                tracing::warn!("PostgreSQL connection attempt {}/5 failed: {}", i, e);
                if i < 5 {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }
    tracing::error!("Failed to connect to PostgreSQL after 5 attempts: {}", last_err);
    std::process::exit(1);
}
