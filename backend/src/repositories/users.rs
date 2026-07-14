use sqlx::{Executor, Postgres};
use uuid::Uuid;

use crate::{
    errors::{AppError, ErrorMessage},
    models::user::{NewUser, PrivacyLevel, User},
};

pub async fn create<'c, E>(executor: E, new_user: NewUser<'_>) -> Result<Uuid, AppError>
where
    E: Executor<'c, Database = Postgres>,
{
    let user = sqlx::query_scalar!("INSERT INTO users (username, email, password_hash, google_id) VALUES ($1, $2, $3, $4) RETURNING id", new_user.username, new_user.email, new_user.password_hash, new_user.google_id).fetch_one(executor).await.map_err(|e| {
		if let sqlx::Error::Database(ref db_err) = e
			&& db_err.code().as_deref() == Some("23505")
		{
			let error_msg = db_err.message();
			if error_msg.contains("email") {
				return AppError::Conflict(ErrorMessage::EmailTaken);
			} else if error_msg.contains("username") {
				return AppError::Conflict(ErrorMessage::UsernameTaken);
			}
		}
		AppError::Database(e)
	})?;
    Ok(user)
}

pub async fn find_by_id<'c, E>(executor: E, user_id: Uuid) -> Result<Option<User>, AppError>
where
    E: Executor<'c, Database = Postgres>,
{
    let user = sqlx::query_as!(User, "SELECT id, username, email, password_hash, email_confirmed, google_id, favorite_genres, privacy_level as \"privacy_level: PrivacyLevel\", is_subscribed, end_subscription_date FROM users WHERE id = $1", user_id).fetch_optional(executor).await.map_err(AppError::Database)?;
    Ok(user)
}

pub async fn find_by_email<'c, E>(executor: E, email: &str) -> Result<Option<User>, AppError>
where
    E: Executor<'c, Database = Postgres>,
{
    let user = sqlx::query_as!(User, "SELECT id, username, email, password_hash, email_confirmed, google_id, favorite_genres, privacy_level as \"privacy_level: PrivacyLevel\", is_subscribed, end_subscription_date FROM users WHERE email = $1", email).fetch_optional(executor).await.map_err(AppError::Database)?;
    Ok(user)
}

pub async fn update_username<'c, E>(
    executor: E,
    user_id: Uuid,
    new_username: &str,
) -> Result<User, AppError>
where
    E: Executor<'c, Database = Postgres>,
{
    let user = sqlx::query_as!(User, "UPDATE users SET username = $1 WHERE id = $2 RETURNING id, username, email, password_hash, email_confirmed, google_id, favorite_genres, privacy_level as \"privacy_level: PrivacyLevel\", is_subscribed, end_subscription_date", new_username, user_id).fetch_one(executor).await.map_err(|e| {
		if let sqlx::Error::Database(ref db_err) = e
				&& db_err.code().as_deref() == Some("23505")
		{
				return AppError::Conflict(ErrorMessage::UsernameTaken);
		}
		AppError::Database(e)
	})?;

    Ok(user)
}

pub async fn update_email<'c, E>(
    executor: E,
    user_id: Uuid,
    new_email: &str,
) -> Result<User, AppError>
where
    E: Executor<'c, Database = Postgres>,
{
    let user = sqlx::query_as!(User, "UPDATE users SET email = $1, email_confirmed = FALSE WHERE id = $2 RETURNING id, username, email, password_hash, email_confirmed, google_id, favorite_genres, privacy_level as \"privacy_level: PrivacyLevel\", is_subscribed, end_subscription_date", new_email, user_id).fetch_one(executor).await.map_err(|e| {
		if let sqlx::Error::Database(ref db_err) = e
				&& db_err.code().as_deref() == Some("23505")
		{
				return AppError::Conflict(ErrorMessage::EmailTaken);
		}
		AppError::Database(e)
	})?;

    Ok(user)
}

pub async fn update_password<'c, E>(
    executor: E,
    user_id: Uuid,
    password_hash: String,
) -> Result<(), AppError>
where
    E: Executor<'c, Database = Postgres>,
{
    sqlx::query!(
        "UPDATE users SET password_hash = $1 WHERE id = $2",
        password_hash,
        user_id
    )
    .execute(executor)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn confirm_email<'c, E>(executor: E, user_id: Uuid) -> Result<(), AppError>
where
    E: Executor<'c, Database = Postgres>,
{
    sqlx::query!(
        "UPDATE users SET email_confirmed = TRUE WHERE id = $1",
        user_id
    )
    .execute(executor)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn confirm_and_update_email<'c, E>(
    executor: E,
    user_id: Uuid,
    new_email: &str,
) -> Result<(), AppError>
where
    E: Executor<'c, Database = Postgres>,
{
    sqlx::query!(
        "UPDATE users SET email = $1, email_confirmed = TRUE WHERE id = $2",
        new_email,
        user_id
    )
    .execute(executor)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn update_favorite_genres<'c, E>(
    executor: E,
    user_id: Uuid,
    favorite_genres: Vec<String>,
) -> Result<User, AppError>
where
    E: Executor<'c, Database = Postgres>,
{
    let user = sqlx::query_as!(
        User,
        r#"
        UPDATE users 
        SET 
            favorite_genres = $1
        WHERE id = $2 
        RETURNING id, username, email, password_hash, email_confirmed, google_id, favorite_genres, privacy_level as "privacy_level: PrivacyLevel", is_subscribed, end_subscription_date
        "#,
        favorite_genres.as_slice(),
        user_id
    )
    .fetch_one(executor)
    .await
    .map_err(AppError::Database)?;

    Ok(user)
}
pub async fn update_privacy_level<'c, E>(
    executor: E,
    user_id: Uuid,
    privacy_level: PrivacyLevel,
) -> Result<User, AppError>
where
    E: Executor<'c, Database = Postgres>,
{
    let user = sqlx::query_as!(
        User,
        r#"
        UPDATE users 
        SET 
            privacy_level = $1::privacy_level
        WHERE id = $2 
        RETURNING id, username, email, password_hash, email_confirmed, google_id, favorite_genres, privacy_level as "privacy_level: PrivacyLevel", is_subscribed, end_subscription_date
        "#,
        privacy_level as PrivacyLevel,
        user_id
    )
    .fetch_one(executor)
    .await
    .map_err(AppError::Database)?;

    Ok(user)
}
pub async fn link_google_id<'c, E>(
    executor: E,
    user_id: Uuid,
    google_id: &str,
) -> Result<(), AppError>
where
    E: Executor<'c, Database = Postgres>,
{
    sqlx::query!(
        "UPDATE users SET google_id = $1 WHERE id = $2",
        google_id,
        user_id
    )
    .execute(executor)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

pub async fn find_by_username<'c, E>(executor: E, username: &str) -> Result<Option<User>, AppError>
where
    E: sqlx::Executor<'c, Database = sqlx::Postgres>,
{
    let user = sqlx::query_as!(User,
        r#"SELECT id, username, email, password_hash, email_confirmed, google_id, favorite_genres, privacy_level as "privacy_level: PrivacyLevel", is_subscribed, end_subscription_date FROM users WHERE username = $1"#,
        username
    )
        .fetch_optional(executor)
        .await
        .map_err(AppError::Database)?;

    Ok(user)
}

pub async fn enable_subscription<'c, E>(executor: E, user_id: Uuid) -> Result<User, AppError>
where
    E: Executor<'c, Database = Postgres>,
{
    let user = sqlx::query_as!(User,
        r#"UPDATE users SET (is_subscribed, end_subscription_date) = (TRUE, NOW() + INTERVAL '1 month') WHERE id = $1 RETURNING id, username, email, password_hash, email_confirmed, google_id, favorite_genres, privacy_level as "privacy_level: PrivacyLevel", is_subscribed, end_subscription_date"#,
        user_id
    )
        .fetch_one(executor)
        .await
        .map_err(AppError::Database)?;

    Ok(user)
}

pub async fn deactivate_subscription<'c, E>(executor: E, user_id: Uuid) -> Result<User, AppError>
where
    E: Executor<'c, Database = Postgres>,
{
    let user = sqlx::query_as!(
        User,
        r#"
        UPDATE users
        SET is_subscribed = FALSE
        WHERE id = $1
        RETURNING id, username, email, password_hash, email_confirmed, google_id, favorite_genres, privacy_level as "privacy_level: PrivacyLevel", is_subscribed, end_subscription_date
        "#,
        user_id
    )
    .fetch_one(executor)
    .await
    .map_err(AppError::Database)?;

    Ok(user)
}
