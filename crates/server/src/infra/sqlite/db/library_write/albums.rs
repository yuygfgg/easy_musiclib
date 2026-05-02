use anyhow::Result;
use easy_musiclib_media::extract_year;
use easy_musiclib_media::normalize::normalize_name;
use sqlx::{Row, SqlitePool};
use std::collections::BTreeSet;
use uuid::Uuid;

pub async fn find_or_create_album(
    pool: &SqlitePool,
    title: &str,
    album_artist_ids: &[i64],
    year: Option<i64>,
    date: Option<&str>,
    event_id: Option<i64>,
    artwork_id: Option<i64>,
) -> Result<i64> {
    let title = title.trim();
    let title = if title.is_empty() {
        "Unknown Album"
    } else {
        title
    };
    let date_year = extract_year(date);
    let title_norm = normalize_name(title, false);
    let candidates = sqlx::query(
        "SELECT id FROM albums
         WHERE title_norm = ?
           AND (year = ? OR year IS NULL OR ? IS NULL)
         ORDER BY id",
    )
    .bind(&title_norm)
    .bind(year)
    .bind(year)
    .fetch_all(pool)
    .await?;
    let wanted: BTreeSet<i64> = album_artist_ids.iter().copied().collect();
    for row in candidates {
        let id: i64 = row.try_get("id")?;
        let existing: BTreeSet<i64> =
            sqlx::query("SELECT artist_id FROM album_artists WHERE album_id = ?")
                .bind(id)
                .fetch_all(pool)
                .await?
                .into_iter()
                .map(|r| r.try_get("artist_id"))
                .collect::<Result<BTreeSet<i64>, sqlx::Error>>()?;
        if existing.is_empty() || wanted.is_empty() || existing == wanted {
            sqlx::query(
                "UPDATE albums SET
                    date = CASE
                      WHEN date IS NULL AND ? IS NOT NULL AND (year IS NULL OR (? IS NOT NULL AND year = ?))
                      THEN ?
                      ELSE date
                    END,
                    year = COALESCE(year, ?),
                    event_id = COALESCE(event_id, ?),
                    artwork_id = COALESCE(artwork_id, ?)
                 WHERE id = ?",
            )
            .bind(date)
            .bind(date_year)
            .bind(date_year)
            .bind(date)
            .bind(year)
            .bind(event_id)
            .bind(artwork_id)
            .bind(id)
            .execute(pool)
            .await?;
            return Ok(id);
        }
    }

    let res = sqlx::query(
        "INSERT INTO albums (uuid, title, title_norm, date, year, event_id, artwork_id)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(title)
    .bind(title_norm)
    .bind(date)
    .bind(year)
    .bind(event_id)
    .bind(artwork_id)
    .execute(pool)
    .await?;
    let id = res.last_insert_rowid();
    for (pos, artist_id) in album_artist_ids.iter().enumerate() {
        sqlx::query(
            "INSERT OR IGNORE INTO album_artists (album_id, artist_id, position) VALUES (?, ?, ?)",
        )
        .bind(id)
        .bind(artist_id)
        .bind(pos as i64)
        .execute(pool)
        .await?;
    }
    if let Some(event_id) = event_id {
        sqlx::query("INSERT OR IGNORE INTO event_albums (event_id, album_id) VALUES (?, ?)")
            .bind(event_id)
            .bind(id)
            .execute(pool)
            .await?;
    }
    Ok(id)
}
