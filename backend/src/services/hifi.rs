use reqwest::Client;

use crate::{
    dtos::hifi::{SearchResponse, TrackResponse},
    errors::AppError,
};

pub async fn search_tracks(query: &str) -> Result<SearchResponse, AppError> {
    let client = Client::new();

    let url = format!("http://hifi-api:8000/search/?s={}", query);

    let response = client
        .get(&url)
        .send()
        .await?
        .json::<SearchResponse>()
        .await?;

    Ok(response)
}

pub async fn get_track_details(track_id: i64) -> Result<TrackResponse, AppError> {
    let client = Client::new();

    let url = format!("http://hifi-api:8000/track/?id={}", track_id);

    let response = client
        .get(&url)
        .send()
        .await?
        .json::<TrackResponse>()
        .await?;

    Ok(response)
}
