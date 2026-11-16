use crate::{errors::AppError, repositories::tracks as tracks_repo, state::AppState};

/// Makes sure the metadata of a track is stored locally, fetching it from the
/// music provider the first time. Room states are built from this cache only.
pub async fn ensure_cached(state: &AppState, track_id: i64) -> Result<(), AppError> {
    if tracks_repo::exists(&state.pool, track_id).await? {
        return Ok(());
    }

    let track = state.music_provider.get_track_info(track_id).await?;
    tracks_repo::insert(&state.pool, &track).await
}
