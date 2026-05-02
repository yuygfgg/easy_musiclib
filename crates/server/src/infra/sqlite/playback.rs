use crate::application::playback::PlaybackRepository;
use crate::domain::{PlaybackSource, TrackId};
use anyhow::{Result, anyhow};
use futures::FutureExt;
use futures::future::BoxFuture;
use sqlx::{Row, SqlitePool};

#[derive(Clone)]
pub struct SqlitePlaybackRepository {
    pool: SqlitePool,
}

impl SqlitePlaybackRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl PlaybackRepository for SqlitePlaybackRepository {
    fn resolve_track_id<'a>(&'a self, ident: &'a str) -> BoxFuture<'a, Result<TrackId>> {
        async move { resolve_track_id(&self.pool, ident).await }.boxed()
    }

    fn track_render_source(&self, track_id: TrackId) -> BoxFuture<'_, Result<PlaybackSource>> {
        async move { track_render_source(&self.pool, track_id).await }.boxed()
    }
}

async fn resolve_track_id(pool: &SqlitePool, ident: &str) -> Result<TrackId> {
    if let Ok(id) = ident.parse::<i64>() {
        if let Some(row) = sqlx::query("SELECT id FROM tracks WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?
        {
            return Ok(TrackId::new(row.try_get("id")?));
        }
    }
    if let Some(row) = sqlx::query("SELECT id FROM tracks WHERE uuid = ?")
        .bind(ident)
        .fetch_optional(pool)
        .await?
    {
        return Ok(TrackId::new(row.try_get("id")?));
    }
    Err(anyhow!("tracks not found: {ident}"))
}

async fn track_render_source(pool: &SqlitePool, track_id: TrackId) -> Result<PlaybackSource> {
    let row = sqlx::query(
        "SELECT t.id, t.title, t.track_no, t.date, al.title AS album_title,
                mf.path, tas.renderer, tas.codec, tas.start_sample, tas.end_sample,
                tas.start_ms, tas.end_ms
         FROM tracks t
         JOIN track_audio_sources tas ON tas.track_id = t.id
         JOIN media_files mf ON mf.id = tas.media_file_id
         LEFT JOIN albums al ON al.id = t.album_id
         WHERE t.id = ?",
    )
    .bind(track_id.raw())
    .fetch_one(pool)
    .await?;
    Ok(PlaybackSource {
        title: row.try_get("title")?,
        artist: track_artist_names(pool, track_id).await?.join(", "),
        album: row.try_get("album_title")?,
        track_no: row.try_get("track_no")?,
        date: row.try_get("date")?,
        path: row.try_get("path")?,
        renderer: row.try_get("renderer")?,
        codec: row.try_get("codec")?,
        start_sample: row.try_get("start_sample")?,
        end_sample: row.try_get("end_sample")?,
        start_ms: row.try_get("start_ms")?,
        end_ms: row.try_get("end_ms")?,
    })
}

async fn track_artist_names(pool: &SqlitePool, track_id: TrackId) -> Result<Vec<String>> {
    let rows = sqlx::query(
        "SELECT a.name
         FROM artists a
         JOIN track_artists ta ON ta.artist_id = a.id
         WHERE ta.track_id = ?
         ORDER BY ta.position, a.name",
    )
    .bind(track_id.raw())
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|row| row.try_get("name").map_err(Into::into))
        .collect()
}
