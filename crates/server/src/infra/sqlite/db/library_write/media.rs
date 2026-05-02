use crate::infra::sqlite::db::now_ms;
use anyhow::Result;
use sqlx::{Row, SqlitePool};

pub async fn upsert_media_file(
    pool: &SqlitePool,
    path: &str,
    path_hash: &str,
    size: i64,
    mtime_ns: i64,
    format: &str,
) -> Result<(i64, bool)> {
    if let Some(row) = sqlx::query("SELECT id, size, mtime_ns FROM media_files WHERE path_hash = ?")
        .bind(path_hash)
        .fetch_optional(pool)
        .await?
    {
        let id: i64 = row.try_get("id")?;
        let old_size: i64 = row.try_get("size")?;
        let old_mtime: i64 = row.try_get("mtime_ns")?;
        let changed = old_size != size || old_mtime != mtime_ns;
        sqlx::query(
            "UPDATE media_files
             SET path = ?, size = ?, mtime_ns = ?, format = ?, last_scanned_at = ?, missing = 0,
                 scan_error = CASE WHEN ? THEN NULL ELSE scan_error END
             WHERE id = ?",
        )
        .bind(path)
        .bind(size)
        .bind(mtime_ns)
        .bind(format)
        .bind(now_ms())
        .bind(changed)
        .bind(id)
        .execute(pool)
        .await?;
        Ok((id, changed))
    } else {
        let res = sqlx::query(
            "INSERT INTO media_files
             (path, path_hash, size, mtime_ns, format, last_scanned_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(path)
        .bind(path_hash)
        .bind(size)
        .bind(mtime_ns)
        .bind(format)
        .bind(now_ms())
        .execute(pool)
        .await?;
        Ok((res.last_insert_rowid(), true))
    }
}

pub async fn delete_tracks_for_media_file(pool: &SqlitePool, media_file_id: i64) -> Result<()> {
    let ids = sqlx::query("SELECT track_id FROM track_audio_sources WHERE media_file_id = ?")
        .bind(media_file_id)
        .fetch_all(pool)
        .await?;
    for row in ids {
        let id: i64 = row.try_get("track_id")?;
        sqlx::query("DELETE FROM search_index WHERE kind = 'track' AND entity_id = ?")
            .bind(id)
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM tracks WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn delete_cue_sheet_for_file(pool: &SqlitePool, cue_file_id: i64) -> Result<()> {
    let sheets = sqlx::query("SELECT id FROM cue_sheets WHERE cue_file_id = ?")
        .bind(cue_file_id)
        .fetch_all(pool)
        .await?;
    for row in sheets {
        let sheet_id: i64 = row.try_get("id")?;
        let track_ids =
            sqlx::query("SELECT track_id FROM track_audio_sources WHERE cue_sheet_id = ?")
                .bind(sheet_id)
                .fetch_all(pool)
                .await?;
        for tr in track_ids {
            let track_id: i64 = tr.try_get("track_id")?;
            sqlx::query("DELETE FROM search_index WHERE kind = 'track' AND entity_id = ?")
                .bind(track_id)
                .execute(pool)
                .await
                .ok();
            sqlx::query("DELETE FROM tracks WHERE id = ?")
                .bind(track_id)
                .execute(pool)
                .await?;
        }
        sqlx::query("DELETE FROM cue_sheets WHERE id = ?")
            .bind(sheet_id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn insert_cue_sheet(
    pool: &SqlitePool,
    cue_file_id: i64,
    audio_file_id: i64,
    album_title: Option<&str>,
    performer: Option<&str>,
    date: Option<&str>,
) -> Result<i64> {
    let res = sqlx::query(
        "INSERT INTO cue_sheets
         (cue_file_id, audio_file_id, album_title, performer, date)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(cue_file_id)
    .bind(audio_file_id)
    .bind(album_title)
    .bind(performer)
    .bind(date)
    .execute(pool)
    .await?;
    Ok(res.last_insert_rowid())
}
