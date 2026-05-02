use crate::infra::sqlite::db::{fetch_artist_summary, refresh_artist_search};
use anyhow::{Result, anyhow};
use easy_musiclib_media::normalize::normalize_name;
use easy_musiclib_shared::ArtistSummary;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

pub async fn create_artist(pool: &SqlitePool, name: &str) -> Result<ArtistSummary> {
    let id = ensure_artist(pool, name, None).await?;
    refresh_artist_search(pool, id).await?;
    fetch_artist_summary(pool, id).await
}

pub async fn ensure_artist(pool: &SqlitePool, name: &str, artwork_id: Option<i64>) -> Result<i64> {
    let name = name.trim();
    if name.is_empty() {
        return Err(anyhow!("artist name is empty"));
    }
    let norm = normalize_name(name, false);
    if let Some(row) = sqlx::query("SELECT id FROM artists WHERE name_norm = ?")
        .bind(&norm)
        .fetch_optional(pool)
        .await?
    {
        let id = row.try_get("id")?;
        if artwork_id.is_some() {
            sqlx::query("UPDATE artists SET artwork_id = COALESCE(artwork_id, ?) WHERE id = ?")
                .bind(artwork_id)
                .bind(id)
                .execute(pool)
                .await?;
        }
        return Ok(id);
    }
    let res =
        sqlx::query("INSERT INTO artists (uuid, name, name_norm, artwork_id) VALUES (?, ?, ?, ?)")
            .bind(Uuid::new_v4().to_string())
            .bind(name)
            .bind(norm)
            .bind(artwork_id)
            .execute(pool)
            .await?;
    Ok(res.last_insert_rowid())
}
