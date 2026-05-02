use super::{fetch_album_summary, fetch_artist_summary, fetch_event_summary, fetch_track_detail};
use anyhow::Result;
use sqlx::{Row, SqlitePool};

pub async fn refresh_track_search(pool: &SqlitePool, track_id: i64) -> Result<()> {
    let detail = fetch_track_detail(pool, track_id).await?;
    sqlx::query("DELETE FROM search_index WHERE kind = 'track' AND entity_id = ?")
        .bind(track_id)
        .execute(pool)
        .await?;
    let artists = detail
        .summary
        .artists
        .iter()
        .map(|a| a.name.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    sqlx::query(
        "INSERT INTO search_index (kind, entity_id, title, artists, album, event, aliases)
         VALUES ('track', ?, ?, ?, ?, ?, '')",
    )
    .bind(track_id)
    .bind(&detail.summary.title)
    .bind(artists)
    .bind(detail.summary.album.as_ref().map(|a| a.name.as_str()))
    .bind(detail.summary.event.as_ref().map(|e| e.name.as_str()))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn refresh_album_search(pool: &SqlitePool, album_id: i64) -> Result<()> {
    let summary = fetch_album_summary(pool, album_id).await?;
    sqlx::query("DELETE FROM search_index WHERE kind = 'album' AND entity_id = ?")
        .bind(album_id)
        .execute(pool)
        .await?;
    let artists = summary
        .album_artists
        .iter()
        .map(|a| a.name.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    sqlx::query(
        "INSERT INTO search_index (kind, entity_id, title, artists, album, event, aliases)
         VALUES ('album', ?, ?, ?, '', ?, '')",
    )
    .bind(album_id)
    .bind(summary.title)
    .bind(artists)
    .bind(summary.event.as_ref().map(|e| e.name.as_str()))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn refresh_artist_search(pool: &SqlitePool, artist_id: i64) -> Result<()> {
    let summary = fetch_artist_summary(pool, artist_id).await?;
    let aliases = sqlx::query("SELECT alias FROM artist_aliases WHERE artist_id = ?")
        .bind(artist_id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .filter_map(|r| r.try_get::<String, _>("alias").ok())
        .collect::<Vec<_>>()
        .join(" ");
    sqlx::query("DELETE FROM search_index WHERE kind = 'artist' AND entity_id = ?")
        .bind(artist_id)
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO search_index (kind, entity_id, title, artists, album, event, aliases)
         VALUES ('artist', ?, ?, '', '', '', ?)",
    )
    .bind(artist_id)
    .bind(summary.name)
    .bind(aliases)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn refresh_event_search(pool: &SqlitePool, event_id: i64) -> Result<()> {
    let summary = fetch_event_summary(pool, event_id).await?;
    sqlx::query("DELETE FROM search_index WHERE kind = 'event' AND entity_id = ?")
        .bind(event_id)
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO search_index (kind, entity_id, title, artists, album, event, aliases)
         VALUES ('event', ?, ?, '', '', ?, '')",
    )
    .bind(event_id)
    .bind(&summary.name)
    .bind(&summary.name)
    .execute(pool)
    .await?;
    Ok(())
}
