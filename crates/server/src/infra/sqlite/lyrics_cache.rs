use crate::application::lyrics::LyricsCacheRepository;
use crate::domain::{LyricsCandidate, TrackId};
use anyhow::Result;
use easy_musiclib_shared as api;
use futures::FutureExt;
use futures::future::BoxFuture;
use sqlx::{Row, SqlitePool};

#[derive(Clone)]
pub struct SqliteLyricsCacheRepository {
    pool: SqlitePool,
}

impl SqliteLyricsCacheRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl LyricsCacheRepository for SqliteLyricsCacheRepository {
    fn cached_lyrics<'a>(
        &'a self,
        track_id: Option<TrackId>,
        title: &'a str,
        artist: &'a str,
    ) -> BoxFuture<'a, Result<Vec<LyricsCandidate>>> {
        async move { cached_lyrics(&self.pool, track_id, title, artist).await }.boxed()
    }

    fn cache_lyrics<'a>(
        &'a self,
        track_id: Option<TrackId>,
        candidate: &'a LyricsCandidate,
    ) -> BoxFuture<'a, Result<()>> {
        async move { cache_lyrics(&self.pool, track_id, candidate).await }.boxed()
    }
}

async fn cached_lyrics(
    pool: &SqlitePool,
    track_id: Option<TrackId>,
    title: &str,
    artist: &str,
) -> Result<Vec<LyricsCandidate>> {
    let rows = if let Some(track_id) = track_id {
        sqlx::query(
            "SELECT title, artist, album, duration_ms, provider, lyrics, score
             FROM lyric_cache WHERE track_id = ? ORDER BY score DESC LIMIT 9",
        )
        .bind(track_id.raw())
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT title, artist, album, duration_ms, provider, lyrics, score
             FROM lyric_cache WHERE title = ? AND artist = ? ORDER BY score DESC LIMIT 9",
        )
        .bind(title)
        .bind(artist)
        .fetch_all(pool)
        .await?
    };

    rows.into_iter()
        .map(|row| {
            Ok(api::LyricsCandidate {
                title: row.try_get("title")?,
                artist: row.try_get("artist")?,
                album: row.try_get("album")?,
                duration_ms: row.try_get("duration_ms")?,
                provider: row.try_get("provider")?,
                lyrics: row.try_get("lyrics")?,
                score: row.try_get("score")?,
            })
            .map(Into::into)
        })
        .collect()
}

async fn cache_lyrics(
    pool: &SqlitePool,
    track_id: Option<TrackId>,
    candidate: &LyricsCandidate,
) -> Result<()> {
    let candidate: api::LyricsCandidate = candidate.clone().into();
    sqlx::query(
        "INSERT INTO lyric_cache
         (track_id, title, artist, album, duration_ms, provider, lyrics, score, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(track_id.map(TrackId::raw))
    .bind(&candidate.title)
    .bind(&candidate.artist)
    .bind(&candidate.album)
    .bind(candidate.duration_ms)
    .bind(&candidate.provider)
    .bind(&candidate.lyrics)
    .bind(candidate.score)
    .bind(now_ms())
    .execute(pool)
    .await?;
    Ok(())
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
