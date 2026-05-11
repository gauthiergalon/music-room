use sqlx::PgPool;

pub struct TrackRecord {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub cover: Option<String>,
    pub duration: i32,
}

pub async fn get_track(pool: &PgPool, track_id: i64) -> sqlx::Result<Option<TrackRecord>> {
    sqlx::query_as!(
        TrackRecord,
        "SELECT title, artist, album, cover, duration FROM tracks WHERE id = $1",
        track_id
    )
    .fetch_optional(pool)
    .await
}
