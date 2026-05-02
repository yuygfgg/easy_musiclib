use crate::application::track_duration::{TrackDurationRepository, TrackDurationSource};
use crate::domain::TrackId;
use anyhow::Result;
use futures::FutureExt;
use futures::future::BoxFuture;
use sqlx::{Row, SqlitePool};

#[derive(Clone)]
pub struct SqliteTrackDurationRepository {
    pool: SqlitePool,
}

impl SqliteTrackDurationRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl TrackDurationRepository for SqliteTrackDurationRepository {
    fn track_duration_source(
        &self,
        id: TrackId,
    ) -> BoxFuture<'_, Result<Option<TrackDurationSource>>> {
        async move { track_duration_source(&self.pool, id).await }.boxed()
    }

    fn persist_track_duration_ms(
        &self,
        source: TrackDurationSource,
        duration_ms: i64,
    ) -> BoxFuture<'_, Result<()>> {
        async move { persist_track_duration_ms(&self.pool, &source, duration_ms).await }.boxed()
    }
}

async fn track_duration_source(
    pool: &SqlitePool,
    id: TrackId,
) -> Result<Option<TrackDurationSource>> {
    let row = sqlx::query(
        "SELECT
            t.duration_ms AS track_duration_ms,
            tas.kind, tas.media_file_id, tas.sample_rate, tas.start_sample, tas.end_sample,
            tas.start_ms, tas.end_ms,
            mf.path, mf.duration_ms AS media_duration_ms
         FROM tracks t
         LEFT JOIN track_audio_sources tas ON tas.track_id = t.id
         LEFT JOIN media_files mf ON mf.id = tas.media_file_id
         WHERE t.id = ?",
    )
    .bind(id.raw())
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(TrackDurationSource {
        track_id: id,
        track_duration_ms: row.try_get("track_duration_ms")?,
        kind: row.try_get("kind")?,
        media_file_id: row.try_get("media_file_id")?,
        path: row.try_get("path")?,
        media_duration_ms: row.try_get("media_duration_ms")?,
        sample_rate: row.try_get("sample_rate")?,
        start_sample: row.try_get("start_sample")?,
        end_sample: row.try_get("end_sample")?,
        start_ms: row.try_get("start_ms")?,
        end_ms: row.try_get("end_ms")?,
    }))
}

async fn persist_track_duration_ms(
    pool: &SqlitePool,
    source: &TrackDurationSource,
    duration_ms: i64,
) -> Result<()> {
    sqlx::query(
        "UPDATE tracks
         SET duration_ms = ?
         WHERE id = ? AND duration_ms IS NULL",
    )
    .bind(duration_ms)
    .bind(source.track_id.raw())
    .execute(pool)
    .await?;

    if source.kind.as_deref() == Some("cue") {
        if source.end_ms.is_none() {
            if let Some(start_ms) = source
                .start_ms
                .or_else(|| cue_start_ms_from_samples(source))
            {
                sqlx::query(
                    "UPDATE track_audio_sources
                     SET start_ms = COALESCE(start_ms, ?), end_ms = ?
                     WHERE track_id = ? AND end_ms IS NULL",
                )
                .bind(start_ms)
                .bind(start_ms.saturating_add(duration_ms))
                .bind(source.track_id.raw())
                .execute(pool)
                .await?;
            }
        }
    } else if source.media_duration_ms.is_none() {
        if let Some(media_file_id) = source.media_file_id {
            sqlx::query(
                "UPDATE media_files
                 SET duration_ms = ?
                 WHERE id = ? AND duration_ms IS NULL",
            )
            .bind(duration_ms)
            .bind(media_file_id)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

fn cue_start_ms_from_samples(source: &TrackDurationSource) -> Option<i64> {
    let sample_rate = source.sample_rate?;
    let start_sample = source.start_sample?;
    (sample_rate > 0).then_some(start_sample.saturating_mul(1000) / sample_rate)
}
