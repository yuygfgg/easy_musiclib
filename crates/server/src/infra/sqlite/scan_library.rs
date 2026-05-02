use crate::application::scan::{NewTrack, NewTrackAudioSource, ScanLibraryRepository};
use crate::infra::sqlite::db;
use anyhow::Result;
use futures::FutureExt;
use futures::future::BoxFuture;
use sqlx::{Row, SqlitePool};

#[derive(Clone)]
pub struct SqliteScanLibraryRepository {
    pool: SqlitePool,
}

impl SqliteScanLibraryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl ScanLibraryRepository for SqliteScanLibraryRepository {
    fn ensure_default_split_exceptions(&self) -> BoxFuture<'_, Result<()>> {
        async move { ensure_default_split_exceptions(&self.pool).await }.boxed()
    }

    fn split_exceptions(&self) -> BoxFuture<'_, Result<Vec<String>>> {
        async move { split_exceptions(&self.pool).await }.boxed()
    }

    fn upsert_media_file<'a>(
        &'a self,
        path: &'a str,
        path_hash: &'a str,
        size: i64,
        mtime_ns: i64,
        format: &'a str,
    ) -> BoxFuture<'a, Result<(i64, bool)>> {
        async move { db::upsert_media_file(&self.pool, path, path_hash, size, mtime_ns, format).await }
            .boxed()
    }

    fn media_file_has_audio_sources(&self, media_file_id: i64) -> BoxFuture<'_, Result<bool>> {
        async move { media_file_has_audio_sources(&self.pool, media_file_id).await }.boxed()
    }

    fn set_media_file_audio_metadata(
        &self,
        media_file_id: i64,
        sample_rate: Option<i64>,
        channels: Option<i64>,
        duration_ms: Option<i64>,
    ) -> BoxFuture<'_, Result<()>> {
        async move {
            set_media_file_audio_metadata(
                &self.pool,
                media_file_id,
                sample_rate,
                channels,
                duration_ms,
            )
            .await
        }
        .boxed()
    }

    fn set_media_file_scan_error<'a>(
        &'a self,
        media_file_id: i64,
        error: &'a str,
    ) -> BoxFuture<'a, Result<()>> {
        async move { set_media_file_scan_error(&self.pool, media_file_id, error).await }.boxed()
    }

    fn delete_tracks_for_media_file(&self, media_file_id: i64) -> BoxFuture<'_, Result<()>> {
        async move { db::delete_tracks_for_media_file(&self.pool, media_file_id).await }.boxed()
    }

    fn delete_cue_sheet_for_file(&self, cue_file_id: i64) -> BoxFuture<'_, Result<()>> {
        async move { db::delete_cue_sheet_for_file(&self.pool, cue_file_id).await }.boxed()
    }

    fn cue_sheet_exists_for_file(&self, cue_file_id: i64) -> BoxFuture<'_, Result<bool>> {
        async move { cue_sheet_exists_for_file(&self.pool, cue_file_id).await }.boxed()
    }

    fn insert_cue_sheet<'a>(
        &'a self,
        cue_file_id: i64,
        audio_file_id: i64,
        album_title: Option<&'a str>,
        performer: Option<&'a str>,
        date: Option<&'a str>,
    ) -> BoxFuture<'a, Result<i64>> {
        async move {
            db::insert_cue_sheet(
                &self.pool,
                cue_file_id,
                audio_file_id,
                album_title,
                performer,
                date,
            )
            .await
        }
        .boxed()
    }

    fn ensure_artist<'a>(
        &'a self,
        name: &'a str,
        artwork_id: Option<i64>,
    ) -> BoxFuture<'a, Result<i64>> {
        async move { db::ensure_artist(&self.pool, name, artwork_id).await }.boxed()
    }

    fn ensure_event<'a>(
        &'a self,
        name: Option<&'a str>,
        date: Option<&'a str>,
        year: Option<i64>,
    ) -> BoxFuture<'a, Result<Option<i64>>> {
        async move { db::ensure_event(&self.pool, name, date, year).await }.boxed()
    }

    fn is_unknown_event_name(&self, name: &str) -> bool {
        db::is_unknown_event_name(name)
    }

    fn find_or_create_album<'a>(
        &'a self,
        title: &'a str,
        album_artist_ids: &'a [i64],
        year: Option<i64>,
        date: Option<&'a str>,
        event_id: Option<i64>,
        artwork_id: Option<i64>,
    ) -> BoxFuture<'a, Result<i64>> {
        async move {
            db::find_or_create_album(
                &self.pool,
                title,
                album_artist_ids,
                year,
                date,
                event_id,
                artwork_id,
            )
            .await
        }
        .boxed()
    }

    fn link_event_album(&self, event_id: i64, album_id: i64) -> BoxFuture<'_, Result<()>> {
        async move { link_event_album(&self.pool, event_id, album_id).await }.boxed()
    }

    fn insert_track<'a>(
        &'a self,
        new_track: NewTrack<'a>,
        artist_ids: &'a [i64],
    ) -> BoxFuture<'a, Result<i64>> {
        async move {
            db::insert_track(
                &self.pool,
                db::NewTrack {
                    title: new_track.title,
                    album_id: new_track.album_id,
                    event_id: new_track.event_id,
                    cue_track_no: new_track.cue_track_no,
                    disc_no: new_track.disc_no,
                    track_no: new_track.track_no,
                    duration_ms: new_track.duration_ms,
                    date: new_track.date,
                    year: new_track.year,
                    artwork_id: new_track.artwork_id,
                },
                artist_ids,
            )
            .await
        }
        .boxed()
    }

    fn insert_track_audio_source<'a>(
        &'a self,
        track_id: i64,
        src: NewTrackAudioSource<'a>,
    ) -> BoxFuture<'a, Result<()>> {
        async move {
            db::insert_track_audio_source(
                &self.pool,
                track_id,
                db::NewTrackAudioSource {
                    kind: src.kind,
                    media_file_id: src.media_file_id,
                    cue_sheet_id: src.cue_sheet_id,
                    codec: src.codec,
                    sample_rate: src.sample_rate,
                    start_sample: src.start_sample,
                    end_sample: src.end_sample,
                    start_ms: src.start_ms,
                    end_ms: src.end_ms,
                    renderer: src.renderer,
                },
            )
            .await
        }
        .boxed()
    }

    fn ensure_artwork_source<'a>(
        &'a self,
        kind: &'a str,
        media_file_id: Option<i64>,
        sidecar_path: Option<&'a str>,
        embedded_picture_index: Option<i64>,
        mime: Option<&'a str>,
    ) -> BoxFuture<'a, Result<Option<i64>>> {
        async move {
            db::ensure_artwork_source(
                &self.pool,
                kind,
                media_file_id,
                sidecar_path,
                embedded_picture_index,
                mime,
            )
            .await
        }
        .boxed()
    }

    fn refresh_track_search(&self, track_id: i64) -> BoxFuture<'_, Result<()>> {
        async move { db::refresh_track_search(&self.pool, track_id).await }.boxed()
    }

    fn refresh_album_search(&self, album_id: i64) -> BoxFuture<'_, Result<()>> {
        async move { db::refresh_album_search(&self.pool, album_id).await }.boxed()
    }

    fn refresh_artist_search(&self, artist_id: i64) -> BoxFuture<'_, Result<()>> {
        async move { db::refresh_artist_search(&self.pool, artist_id).await }.boxed()
    }

    fn refresh_event_search(&self, event_id: i64) -> BoxFuture<'_, Result<()>> {
        async move { db::refresh_event_search(&self.pool, event_id).await }.boxed()
    }

    fn discard_unknown_events(&self) -> BoxFuture<'_, Result<()>> {
        async move { db::discard_unknown_events(&self.pool).await }.boxed()
    }

    fn repair_event_dates_and_artwork(&self) -> BoxFuture<'_, Result<()>> {
        async move { db::repair_event_dates_and_artwork(&self.pool).await }.boxed()
    }

    fn rebuild_relations(&self) -> BoxFuture<'_, Result<()>> {
        async move { db::rebuild_relations(&self.pool).await }.boxed()
    }

    fn auto_merge(&self) -> BoxFuture<'_, Result<usize>> {
        async move { db::auto_merge(&self.pool).await }.boxed()
    }
}

async fn media_file_has_audio_sources(pool: &SqlitePool, media_file_id: i64) -> Result<bool> {
    let row = sqlx::query("SELECT COUNT(*) AS n FROM track_audio_sources WHERE media_file_id = ?")
        .bind(media_file_id)
        .fetch_one(pool)
        .await?;
    Ok(row.try_get::<i64, _>("n")? > 0)
}

async fn set_media_file_audio_metadata(
    pool: &SqlitePool,
    media_file_id: i64,
    sample_rate: Option<i64>,
    channels: Option<i64>,
    duration_ms: Option<i64>,
) -> Result<()> {
    sqlx::query(
        "UPDATE media_files SET scan_error = NULL, sample_rate = ?, channels = ?, duration_ms = ? WHERE id = ?",
    )
    .bind(sample_rate)
    .bind(channels)
    .bind(duration_ms)
    .bind(media_file_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn set_media_file_scan_error(
    pool: &SqlitePool,
    media_file_id: i64,
    error: &str,
) -> Result<()> {
    sqlx::query("UPDATE media_files SET scan_error = ? WHERE id = ?")
        .bind(error)
        .bind(media_file_id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn cue_sheet_exists_for_file(pool: &SqlitePool, cue_file_id: i64) -> Result<bool> {
    Ok(
        sqlx::query("SELECT id FROM cue_sheets WHERE cue_file_id = ?")
            .bind(cue_file_id)
            .fetch_optional(pool)
            .await?
            .is_some(),
    )
}

async fn link_event_album(pool: &SqlitePool, event_id: i64, album_id: i64) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO event_albums (event_id, album_id) VALUES (?, ?)")
        .bind(event_id)
        .bind(album_id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn split_exceptions(pool: &SqlitePool) -> Result<Vec<String>> {
    let rows = sqlx::query("SELECT name FROM artist_split_exceptions ORDER BY id")
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| row.try_get::<String, _>("name").ok())
        .collect())
}

async fn ensure_default_split_exceptions(pool: &SqlitePool) -> Result<()> {
    for name in easy_musiclib_media::artists::DEFAULT_SPLIT_EXCEPTIONS {
        sqlx::query(
            "INSERT OR IGNORE INTO artist_split_exceptions (name, name_norm, created_at)
             VALUES (?, ?, ?)",
        )
        .bind(name)
        .bind(easy_musiclib_media::normalize::normalize_name(name, false))
        .bind(db::now_ms())
        .execute(pool)
        .await?;
    }
    Ok(())
}
