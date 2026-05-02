use crate::infra::sqlite::db::{is_unknown_event_name, refresh_album_search, refresh_track_search};
use anyhow::Result;
use easy_musiclib_media::normalize::normalize_name;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

pub async fn ensure_event(
    pool: &SqlitePool,
    name: Option<&str>,
    date: Option<&str>,
    year: Option<i64>,
) -> Result<Option<i64>> {
    let Some(name) = name.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if is_unknown_event_name(name) {
        return Ok(None);
    }
    let norm = normalize_name(name, false);
    if let Some(row) = sqlx::query("SELECT id, date, year FROM events WHERE name_norm = ?")
        .bind(&norm)
        .fetch_optional(pool)
        .await?
    {
        let id: i64 = row.try_get("id")?;
        update_event_date_year(pool, id, date, year).await?;
        return Ok(Some(id));
    }
    let res = sqlx::query(
        "INSERT INTO events (uuid, name, name_norm, date, year) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(name)
    .bind(norm)
    .bind(date)
    .bind(year)
    .execute(pool)
    .await?;
    Ok(Some(res.last_insert_rowid()))
}

pub async fn update_event_date_year(
    pool: &SqlitePool,
    event_id: i64,
    date: Option<&str>,
    year: Option<i64>,
) -> Result<()> {
    if date.is_some() || year.is_some() {
        sqlx::query(
            "UPDATE events
             SET date = CASE
                    WHEN ? IS NULL THEN date
                    WHEN date IS NULL THEN ?
                    WHEN length(date) <= 4 AND length(?) > length(date) THEN ?
                    ELSE date
                 END,
                 year = COALESCE(year, ?)
             WHERE id = ?",
        )
        .bind(date)
        .bind(date)
        .bind(date)
        .bind(date)
        .bind(year)
        .bind(event_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn discard_unknown_events(pool: &SqlitePool) -> Result<()> {
    let event_ids: Vec<i64> = sqlx::query_scalar(
        "SELECT id FROM events
         WHERE REPLACE(REPLACE(REPLACE(name_norm, ' ', ''), '_', ''), '-', '') = 'unknownevent'",
    )
    .fetch_all(pool)
    .await?;
    if event_ids.is_empty() {
        return Ok(());
    }

    let album_ids: Vec<i64> = sqlx::query_scalar(
        "SELECT id FROM albums
         WHERE event_id IN (
           SELECT id FROM events
           WHERE REPLACE(REPLACE(REPLACE(name_norm, ' ', ''), '_', ''), '-', '') = 'unknownevent'
         )",
    )
    .fetch_all(pool)
    .await?;
    let track_ids: Vec<i64> = sqlx::query_scalar(
        "SELECT id FROM tracks
         WHERE event_id IN (
           SELECT id FROM events
           WHERE REPLACE(REPLACE(REPLACE(name_norm, ' ', ''), '_', ''), '-', '') = 'unknownevent'
         )",
    )
    .fetch_all(pool)
    .await?;

    sqlx::query(
        "UPDATE albums SET event_id = NULL
         WHERE event_id IN (
           SELECT id FROM events
           WHERE REPLACE(REPLACE(REPLACE(name_norm, ' ', ''), '_', ''), '-', '') = 'unknownevent'
         )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE tracks SET event_id = NULL
         WHERE event_id IN (
           SELECT id FROM events
           WHERE REPLACE(REPLACE(REPLACE(name_norm, ' ', ''), '_', ''), '-', '') = 'unknownevent'
         )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "DELETE FROM event_albums
         WHERE event_id IN (
           SELECT id FROM events
           WHERE REPLACE(REPLACE(REPLACE(name_norm, ' ', ''), '_', ''), '-', '') = 'unknownevent'
         )",
    )
    .execute(pool)
    .await?;
    for event_id in event_ids {
        sqlx::query("DELETE FROM search_index WHERE kind = 'event' AND entity_id = ?")
            .bind(event_id)
            .execute(pool)
            .await?;
    }
    sqlx::query(
        "DELETE FROM events
         WHERE REPLACE(REPLACE(REPLACE(name_norm, ' ', ''), '_', ''), '-', '') = 'unknownevent'",
    )
    .execute(pool)
    .await?;

    for album_id in album_ids {
        refresh_album_search(pool, album_id).await?;
    }
    for track_id in track_ids {
        refresh_track_search(pool, track_id).await?;
    }
    Ok(())
}
