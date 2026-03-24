use chrono::Utc;
use sqlx::PgPool;
use std::time::Duration;
use tokio::time;

pub fn spawn_token_cleanup_task(pool: PgPool) {
	tokio::spawn(async move {
		let mut interval = time::interval(Duration::from_secs(60 * 60 * 24));

		loop {
			interval.tick().await;

			let res_refresh = sqlx::query!("DELETE FROM refresh_tokens WHERE expires_at < $1", Utc::now()).execute(&pool).await;

			if let Ok(result) = res_refresh {
				tracing::info!("Cleaned up {} expired refresh tokens", result.rows_affected());
			}

			let res_reset = sqlx::query!("DELETE FROM reset_tokens WHERE expires_at < $1", Utc::now()).execute(&pool).await;

			if let Ok(result) = res_reset {
				tracing::info!("Cleaned up {} expired reset tokens", result.rows_affected());
			}
		}
	});
}
