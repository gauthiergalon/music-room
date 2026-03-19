use crate::errors::{AppError, ErrorMessage};
use crate::models::user::{NewUser, User};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

pub async fn insert<'c, E>(executor: E, new_user: NewUser<'_>) -> Result<Uuid, AppError>
where
	E: Executor<'c, Database = Postgres>,
{
	let id = sqlx::query_scalar!("INSERT INTO users (username, email, password_hash, google_id) VALUES ($1, $2, $3, $4) RETURNING id", new_user.username, new_user.email, new_user.password_hash, new_user.google_id).fetch_one(executor).await.map_err(|e| {
		if let sqlx::Error::Database(ref db_err) = e {
			if db_err.code().as_deref() == Some("23505") {
				let error_msg = db_err.message();
				if error_msg.contains("email") {
					return AppError::Conflict(ErrorMessage::EmailTaken);
				} else if error_msg.contains("username") {
					return AppError::Conflict(ErrorMessage::UsernameTaken);
				}
			}
		}
		AppError::Database(e)
	})?;
	Ok(id)
}

pub async fn find_by_id<'c, E>(executor: E, user_id: Uuid) -> Result<Option<User>, AppError>
where
	E: Executor<'c, Database = Postgres>,
{
	let user = sqlx::query!("SELECT id, username, email, password_hash, google_id FROM users WHERE id = $1", user_id).fetch_optional(executor).await.map_err(AppError::Database)?;
	Ok(user.map(|u| User { id: u.id, username: u.username, email: u.email, password_hash: u.password_hash, google_id: u.google_id }))
}

pub async fn find_by_email<'c, E>(executor: E, email: &str) -> Result<Option<User>, AppError>
where
	E: Executor<'c, Database = Postgres>,
{
	let user = sqlx::query!("SELECT id, username, email, password_hash, google_id FROM users WHERE email = $1", email).fetch_optional(executor).await.map_err(AppError::Database)?;
	Ok(user.map(|u| User { id: u.id, username: u.username, email: u.email, password_hash: u.password_hash, google_id: u.google_id }))
}

pub async fn update_username<'c, E>(executor: E, user_id: Uuid, new_username: &str) -> Result<User, AppError>
where
	E: Executor<'c, Database = Postgres>,
{
	let user = sqlx::query!("UPDATE users SET username = $1 WHERE id = $2 RETURNING id, username, email, password_hash, google_id", new_username, user_id).fetch_one(executor).await.map_err(AppError::Database)?;
	Ok(User { id: user.id, username: user.username, email: user.email, password_hash: user.password_hash, google_id: user.google_id })
}

pub async fn update_email<'c, E>(executor: E, user_id: Uuid, new_email: &str) -> Result<User, AppError>
where
	E: Executor<'c, Database = Postgres>,
{
	let user = sqlx::query!("UPDATE users SET email = $1 WHERE id = $2 RETURNING id, username, email, password_hash, google_id", new_email, user_id).fetch_one(executor).await.map_err(AppError::Database)?;
	Ok(User { id: user.id, username: user.username, email: user.email, password_hash: user.password_hash, google_id: user.google_id })
}

pub async fn update_password<'c, E>(executor: E, user_id: Uuid, password_hash: String) -> Result<(), AppError>
where
	E: Executor<'c, Database = Postgres>,
{
	sqlx::query!("UPDATE users SET password_hash = $1 WHERE id = $2", password_hash, user_id).execute(executor).await.map_err(AppError::Database)?;
	Ok(())
}
