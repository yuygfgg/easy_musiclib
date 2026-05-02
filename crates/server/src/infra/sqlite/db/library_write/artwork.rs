use crate::infra::sqlite::db::now_ms;
use anyhow::Result;
use sqlx::{Row, SqlitePool};

pub async fn ensure_artwork_source(
    pool: &SqlitePool,
    kind: &str,
    media_file_id: Option<i64>,
    sidecar_path: Option<&str>,
    embedded_picture_index: Option<i64>,
    mime: Option<&str>,
) -> Result<Option<i64>> {
    let row = match kind {
        "sidecar" => {
            if sidecar_path.is_none() {
                return Ok(None);
            }
            sqlx::query(
                "SELECT id FROM artwork_sources WHERE kind = 'sidecar' AND sidecar_path = ?",
            )
            .bind(sidecar_path)
            .fetch_optional(pool)
            .await?
        }
        "embedded" => {
            if media_file_id.is_none() {
                return Ok(None);
            }
            sqlx::query(
                "SELECT id FROM artwork_sources
                 WHERE kind = 'embedded' AND media_file_id = ? AND embedded_picture_index = ?",
            )
            .bind(media_file_id)
            .bind(embedded_picture_index.unwrap_or(0))
            .fetch_optional(pool)
            .await?
        }
        _ => None,
    };
    if let Some(row) = row {
        return Ok(Some(row.try_get("id")?));
    }
    let res = sqlx::query(
        "INSERT INTO artwork_sources
         (kind, media_file_id, sidecar_path, embedded_picture_index, mime, last_checked_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(kind)
    .bind(media_file_id)
    .bind(sidecar_path)
    .bind(embedded_picture_index)
    .bind(mime)
    .bind(now_ms())
    .execute(pool)
    .await?;
    Ok(Some(res.last_insert_rowid()))
}
