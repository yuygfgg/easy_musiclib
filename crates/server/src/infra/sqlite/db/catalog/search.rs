use super::albums::list_albums;
use super::artists::list_artists;
use super::events::list_events;
use super::tracks::track_summary_from_row;
use anyhow::Result;
use easy_musiclib_media::normalize::{fuzzy_score, normalize_name};
use easy_musiclib_shared::SearchResponse;
use sqlx::{Row, SqlitePool};

pub async fn search(pool: &SqlitePool, q: &str, limit: i64) -> Result<SearchResponse> {
    let limit = limit.clamp(1, 100);
    let norm = normalize_name(q, true);
    let like = format!("%{norm}%");

    let mut track_rows = sqlx::query(
        "SELECT DISTINCT
            t.id, t.uuid, t.title, t.artwork_id, t.track_no, t.disc_no, t.duration_ms,
            t.year, t.date, t.liked_at, t.cue_track_no,
            al.id AS album_id, al.uuid AS album_uuid, al.title AS album_title,
            ev.id AS event_id, ev.uuid AS event_uuid, ev.name AS event_name,
            tas.renderer AS renderer
         FROM tracks t
         LEFT JOIN albums al ON al.id = t.album_id
         LEFT JOIN events ev ON ev.id = t.event_id
         LEFT JOIN track_audio_sources tas ON tas.track_id = t.id
         WHERE t.title_norm LIKE ?
            OR al.title_norm LIKE ?
            OR ev.name_norm LIKE ?
            OR EXISTS (
              SELECT 1 FROM track_artists ta
              JOIN artists a ON a.id = ta.artist_id
              WHERE ta.track_id = t.id AND a.name_norm LIKE ?
            )
         LIMIT ?",
    )
    .bind(&like)
    .bind(&like)
    .bind(&like)
    .bind(&like)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    track_rows.sort_by(|a, b| {
        let at: String = a.try_get("title").unwrap_or_default();
        let bt: String = b.try_get("title").unwrap_or_default();
        fuzzy_score(&bt, q)
            .partial_cmp(&fuzzy_score(&at, q))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut tracks = Vec::new();
    for row in track_rows {
        tracks.push(track_summary_from_row(pool, row).await?);
    }

    let albums = list_albums(
        pool,
        None,
        None,
        limit,
        None,
        None,
        None,
        Some(q.to_string()),
    )
    .await?
    .items;
    let artists = list_artists(pool, None, None, limit, None, Some(q.to_string()))
        .await?
        .items;
    let events = list_events(pool, None, None, limit, None, Some(q.to_string()))
        .await?
        .items;
    Ok(SearchResponse {
        tracks,
        albums,
        artists,
        events,
    })
}
