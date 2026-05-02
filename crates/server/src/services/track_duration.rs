use crate::{ApiResult, AppState, db};
use easy_musiclib_media::metadata::read_audio_metadata;
use easy_musiclib_shared::TrackDetail;
use sqlx::Row;
use std::path::PathBuf;

pub async fn fetch_track_detail_with_duration(state: &AppState, id: i64) -> ApiResult<TrackDetail> {
    ensure_track_duration_ms(state, id).await?;
    Ok(db::fetch_track_detail(&state.pool, id).await?)
}

pub async fn ensure_track_duration_ms(state: &AppState, id: i64) -> ApiResult<()> {
    let Some(source) = track_duration_source(state, id).await? else {
        return Ok(());
    };
    if source.track_duration_ms.is_some() {
        return Ok(());
    }

    match infer_track_duration_ms(&source).await {
        Ok(Some(duration_ms)) => {
            persist_track_duration_ms(state, &source, duration_ms).await?;
        }
        Ok(None) => {}
        Err(err) => {
            tracing::warn!(
                track_id = id,
                path = %source.path.as_deref().unwrap_or(""),
                error = %err,
                "failed to infer track duration"
            );
        }
    }
    Ok(())
}

async fn track_duration_source(
    state: &AppState,
    id: i64,
) -> ApiResult<Option<TrackDurationSource>> {
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
    .bind(id)
    .fetch_optional(&state.pool)
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

async fn infer_track_duration_ms(source: &TrackDurationSource) -> anyhow::Result<Option<i64>> {
    if let (Some(start_ms), Some(end_ms)) = (source.start_ms, source.end_ms) {
        return Ok(positive_duration(end_ms.saturating_sub(start_ms)));
    }
    if let (Some(sample_rate), Some(start_sample), Some(end_sample)) =
        (source.sample_rate, source.start_sample, source.end_sample)
    {
        if sample_rate > 0 {
            return Ok(positive_duration(
                end_sample.saturating_sub(start_sample).saturating_mul(1000) / sample_rate,
            ));
        }
    }

    let file_duration_ms = match source.media_duration_ms {
        Some(duration_ms) => Some(duration_ms),
        None => read_source_duration_ms(source).await?,
    };
    let Some(file_duration_ms) = file_duration_ms else {
        return Ok(None);
    };
    if source.kind.as_deref() == Some("cue") {
        let start_ms = source
            .start_ms
            .or_else(|| cue_start_ms_from_samples(source))
            .unwrap_or(0);
        return Ok(positive_duration(file_duration_ms.saturating_sub(start_ms)));
    }
    Ok(positive_duration(file_duration_ms))
}

async fn read_source_duration_ms(source: &TrackDurationSource) -> anyhow::Result<Option<i64>> {
    let Some(path) = source.path.clone() else {
        return Ok(None);
    };
    let tags = tokio::task::spawn_blocking(move || {
        let path = PathBuf::from(path);
        read_audio_metadata(&path, &[])
    })
    .await
    .map_err(|e| anyhow::anyhow!(e.to_string()))??;
    Ok(tags.duration_ms)
}

fn cue_start_ms_from_samples(source: &TrackDurationSource) -> Option<i64> {
    let sample_rate = source.sample_rate?;
    let start_sample = source.start_sample?;
    (sample_rate > 0).then_some(start_sample.saturating_mul(1000) / sample_rate)
}

fn positive_duration(duration_ms: i64) -> Option<i64> {
    (duration_ms > 0).then_some(duration_ms)
}

async fn persist_track_duration_ms(
    state: &AppState,
    source: &TrackDurationSource,
    duration_ms: i64,
) -> ApiResult<()> {
    sqlx::query(
        "UPDATE tracks
         SET duration_ms = ?
         WHERE id = ? AND duration_ms IS NULL",
    )
    .bind(duration_ms)
    .bind(source.track_id)
    .execute(&state.pool)
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
                .bind(source.track_id)
                .execute(&state.pool)
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
            .execute(&state.pool)
            .await?;
        }
    }
    Ok(())
}

struct TrackDurationSource {
    track_id: i64,
    track_duration_ms: Option<i64>,
    kind: Option<String>,
    media_file_id: Option<i64>,
    path: Option<String>,
    media_duration_ms: Option<i64>,
    sample_rate: Option<i64>,
    start_sample: Option<i64>,
    end_sample: Option<i64>,
    start_ms: Option<i64>,
    end_ms: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use easy_musiclib_media::cue_render::PASSTHROUGH_RENDERER;
    use easy_musiclib_media::path_hash;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::io::Write;
    use std::sync::Arc;

    #[tokio::test]
    async fn fetch_track_detail_backfills_missing_file_duration() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::schema::init_db(&pool).await.unwrap();

        let temp = tempfile::tempdir().unwrap();
        let wav_path = temp.path().join("legacy.wav");
        write_test_wav(&wav_path, 2, 8_000);

        let metadata = std::fs::metadata(&wav_path).unwrap();
        let (media_file_id, _) = db::upsert_media_file(
            &pool,
            &wav_path.to_string_lossy(),
            &path_hash(&wav_path),
            metadata.len().try_into().unwrap(),
            0,
            "wav",
        )
        .await
        .unwrap();
        let track_id = db::insert_track(
            &pool,
            db::NewTrack {
                title: "Legacy WAV",
                album_id: None,
                event_id: None,
                cue_track_no: None,
                disc_no: None,
                track_no: None,
                duration_ms: None,
                date: None,
                year: None,
                artwork_id: None,
            },
            &[],
        )
        .await
        .unwrap();
        db::insert_track_audio_source(
            &pool,
            track_id,
            db::NewTrackAudioSource {
                kind: "file",
                media_file_id,
                cue_sheet_id: None,
                codec: "wav",
                sample_rate: None,
                start_sample: None,
                end_sample: None,
                start_ms: None,
                end_ms: None,
                renderer: PASSTHROUGH_RENDERER,
            },
        )
        .await
        .unwrap();

        let state = AppState {
            pool: pool.clone(),
            static_dir: Arc::new(temp.path().to_path_buf()),
        };
        let detail = fetch_track_detail_with_duration(&state, track_id)
            .await
            .unwrap();
        let duration_ms = detail.summary.duration_ms.unwrap();
        assert!((1_900..=2_100).contains(&duration_ms));

        let stored: i64 = sqlx::query_scalar("SELECT duration_ms FROM tracks WHERE id = ?")
            .bind(track_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stored, duration_ms);
    }

    fn write_test_wav(path: &std::path::Path, seconds: u32, sample_rate: u32) {
        let channels = 1_u16;
        let bits_per_sample = 16_u16;
        let samples = seconds * sample_rate;
        let data_len = samples * u32::from(channels) * u32::from(bits_per_sample / 8);
        let mut file = std::fs::File::create(path).unwrap();

        file.write_all(b"RIFF").unwrap();
        file.write_all(&(36 + data_len).to_le_bytes()).unwrap();
        file.write_all(b"WAVE").unwrap();
        file.write_all(b"fmt ").unwrap();
        file.write_all(&16_u32.to_le_bytes()).unwrap();
        file.write_all(&1_u16.to_le_bytes()).unwrap();
        file.write_all(&channels.to_le_bytes()).unwrap();
        file.write_all(&sample_rate.to_le_bytes()).unwrap();
        file.write_all(&(sample_rate * u32::from(channels) * 2).to_le_bytes())
            .unwrap();
        file.write_all(&(channels * 2).to_le_bytes()).unwrap();
        file.write_all(&bits_per_sample.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&data_len.to_le_bytes()).unwrap();

        for _ in 0..samples {
            file.write_all(&0_i16.to_le_bytes()).unwrap();
        }
    }
}
