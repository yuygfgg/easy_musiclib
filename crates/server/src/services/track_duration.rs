use crate::application::track_duration as track_duration_app;
use crate::domain::TrackId;
use crate::{ApiResult, AppState};
use easy_musiclib_shared::TrackDetail;

pub async fn fetch_track_detail_with_duration(
    state: &AppState,
    id: TrackId,
) -> ApiResult<TrackDetail> {
    Ok(track_duration_app::fetch_track_detail_with_duration(
        &state.repositories.catalog,
        &state.repositories.track_duration,
        &state.services.audio_metadata_reader,
        id,
    )
    .await?
    .into())
}

pub async fn ensure_track_duration_ms(state: &AppState, id: TrackId) -> ApiResult<()> {
    Ok(track_duration_app::ensure_track_duration_ms(
        &state.repositories.track_duration,
        &state.services.audio_metadata_reader,
        id,
    )
    .await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use easy_musiclib_media::path_hash;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::io::Write;

    const TEST_PASSTHROUGH_RENDERER: &str = "passthrough";

    #[tokio::test]
    async fn fetch_track_detail_backfills_missing_file_duration() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::infra::sqlite::schema::init_db(&pool).await.unwrap();

        let temp = tempfile::tempdir().unwrap();
        let wav_path = temp.path().join("legacy.wav");
        write_test_wav(&wav_path, 2, 8_000);

        let metadata = std::fs::metadata(&wav_path).unwrap();
        let media_file_id: i64 = sqlx::query_scalar(
            "INSERT INTO media_files
             (path, path_hash, size, mtime_ns, format, last_scanned_at)
             VALUES (?, ?, ?, ?, ?, ?)
             RETURNING id",
        )
        .bind(wav_path.to_string_lossy().as_ref())
        .bind(path_hash(&wav_path))
        .bind(i64::try_from(metadata.len()).unwrap())
        .bind(0_i64)
        .bind("wav")
        .bind(0_i64)
        .fetch_one(&pool)
        .await
        .unwrap();
        let track_id: i64 = sqlx::query_scalar(
            "INSERT INTO tracks (uuid, title, title_norm)
             VALUES (?, ?, ?)
             RETURNING id",
        )
        .bind("track-test-uuid")
        .bind("Legacy WAV")
        .bind("legacy wav")
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO track_audio_sources
             (track_id, kind, media_file_id, codec, renderer)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(track_id)
        .bind("file")
        .bind(media_file_id)
        .bind("wav")
        .bind(TEST_PASSTHROUGH_RENDERER)
        .execute(&pool)
        .await
        .unwrap();

        let state = crate::app::state_from_pool(pool.clone(), temp.path().to_path_buf());
        let detail = fetch_track_detail_with_duration(&state, TrackId::new(track_id))
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
