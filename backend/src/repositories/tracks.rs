use sqlx::PgPool;

use crate::{dtos::music::TrackItem, errors::AppError};

pub async fn exists(pool: &PgPool, track_id: i64) -> Result<bool, AppError> {
    let exists = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM tracks WHERE id = $1) AS "exists!""#,
        track_id
    )
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

/// Stores the metadata of a track so room states never need the music provider.
pub async fn insert(pool: &PgPool, track: &TrackItem) -> Result<(), AppError> {
    let artist = track
        .artists
        .as_ref()
        .and_then(|artists| artists.first())
        .and_then(|artist| artist.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());
    let album = track.album.as_ref().and_then(|album| album.title.clone());
    let cover = track.album.as_ref().and_then(|album| album.cover.clone());

    sqlx::query!(
        "INSERT INTO tracks (id, title, artist, album, duration, cover) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (id) DO NOTHING",
        track.id,
        truncate(&track.title, 255),
        truncate(&artist, 255),
        album.as_deref().map(|value| truncate(value, 255)),
        track.duration,
        cover.as_deref().map(|value| truncate(value, 255)),
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// The metadata columns are `VARCHAR(255)`; a longer provider value must not fail the insert.
fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}
