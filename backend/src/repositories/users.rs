use crate::errors::{AppError, ErrorMessage};
use crate::models::user::{NewUser, User};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub async fn insert(pool: &PgPool, new_user: NewUser<'_>) -> Result<Uuid, AppError> {
    let id = sqlx::query_scalar!(
        "INSERT INTO users (username, email, password_hash, google_id) VALUES ($1, $2, $3, $4) RETURNING id",
        new_user.username,
        new_user.email,
        new_user.password_hash,
        new_user.google_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| {
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

pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, AppError> {
    let user = sqlx::query!(
        "SELECT id, username, email, password_hash, google_id FROM users WHERE email = $1",
        email
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(user.map(|u| User {
        id: u.id,
        username: u.username,
        email: u.email,
        password_hash: u.password_hash,
        google_id: u.google_id,
    }))
}

pub async fn update_password<'a>(
    tx: &mut Transaction<'a, Postgres>,
    user_id: Uuid,
    password_hash: String,
) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE users SET password_hash = $1 WHERE id = $2",
        password_hash,
        user_id
    )
    .execute(&mut **tx)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}
