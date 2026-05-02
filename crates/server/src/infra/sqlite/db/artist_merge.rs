use super::relations::rebuild_relations;
use super::search_index::refresh_artist_search;
use super::{ensure_artist, entity_ref, now_ms};
use anyhow::Result;
use easy_musiclib_media::normalize::normalize_name;
use sqlx::{Row, SqlitePool};

pub async fn add_artist_alias(pool: &SqlitePool, artist_id: i64, alias: &str) -> Result<()> {
    let alias = alias.trim();
    if alias.is_empty() {
        return Ok(());
    }
    let norm = normalize_name(alias, false);
    sqlx::query(
        "INSERT OR IGNORE INTO artist_aliases (artist_id, alias, alias_norm) VALUES (?, ?, ?)",
    )
    .bind(artist_id)
    .bind(alias)
    .bind(norm)
    .execute(pool)
    .await?;
    refresh_artist_search(pool, artist_id).await?;
    Ok(())
}

pub async fn merge_artists(
    pool: &SqlitePool,
    target_id: i64,
    source_id: i64,
    reason: &str,
) -> Result<()> {
    if target_id == source_id {
        return Ok(());
    }
    let source = entity_ref(pool, "artists", source_id).await?;
    sqlx::query(
        "UPDATE artists
         SET artwork_id = COALESCE(artwork_id, (SELECT artwork_id FROM artists WHERE id = ?))
         WHERE id = ?",
    )
    .bind(source_id)
    .bind(target_id)
    .execute(pool)
    .await?;
    add_artist_alias(pool, target_id, &source.name).await?;

    sqlx::query(
        "INSERT OR IGNORE INTO track_artists (track_id, artist_id, role, position)
         SELECT track_id, ?, role, position FROM track_artists WHERE artist_id = ?",
    )
    .bind(target_id)
    .bind(source_id)
    .execute(pool)
    .await?;
    sqlx::query("DELETE FROM track_artists WHERE artist_id = ?")
        .bind(source_id)
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT OR IGNORE INTO album_artists (album_id, artist_id, position)
         SELECT album_id, ?, position FROM album_artists WHERE artist_id = ?",
    )
    .bind(target_id)
    .bind(source_id)
    .execute(pool)
    .await?;
    sqlx::query("DELETE FROM album_artists WHERE artist_id = ?")
        .bind(source_id)
        .execute(pool)
        .await?;
    sqlx::query("UPDATE artist_aliases SET artist_id = ? WHERE artist_id = ?")
        .bind(target_id)
        .bind(source_id)
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO artist_merge_audit
         (target_artist_id, source_artist_uuid, source_artist_name, reason, merged_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(target_id)
    .bind(source.uuid)
    .bind(source.name)
    .bind(reason)
    .bind(now_ms())
    .execute(pool)
    .await?;
    sqlx::query("DELETE FROM search_index WHERE kind = 'artist' AND entity_id = ?")
        .bind(source_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM artists WHERE id = ?")
        .bind(source_id)
        .execute(pool)
        .await?;

    refresh_artist_search(pool, target_id).await?;
    rebuild_relations(pool).await?;
    Ok(())
}

pub async fn import_alias_csv(pool: &SqlitePool, csv: &str) -> Result<usize> {
    let mut count = 0;
    for line in csv.lines() {
        let cols = parse_csv_line(line);
        if cols.len() < 2 {
            continue;
        }
        let primary = cols[0].trim();
        if primary.is_empty() {
            continue;
        }
        let target_id = ensure_artist(pool, primary, None).await?;
        for alias in cols
            .iter()
            .skip(1)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            let alias_id = ensure_artist(pool, alias, None).await?;
            if alias_id != target_id {
                merge_artists(pool, target_id, alias_id, "alias_csv").await?;
                count += 1;
            } else {
                add_artist_alias(pool, target_id, alias).await?;
            }
        }
    }
    rebuild_relations(pool).await?;
    Ok(count)
}

pub async fn auto_merge(pool: &SqlitePool) -> Result<usize> {
    let rows = sqlx::query(
        "SELECT aa.artist_id AS target_id, a2.id AS source_id
         FROM artist_aliases aa
         JOIN artists a2 ON a2.name_norm = aa.alias_norm
         WHERE a2.id <> aa.artist_id",
    )
    .fetch_all(pool)
    .await?;
    let mut count = 0;
    for row in rows {
        merge_artists(
            pool,
            row.try_get("target_id")?,
            row.try_get("source_id")?,
            "auto_merge_alias",
        )
        .await?;
        count += 1;
    }
    repair_event_dates_and_artwork(pool).await?;
    rebuild_relations(pool).await?;
    Ok(count)
}

pub async fn repair_event_dates_and_artwork(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "UPDATE albums
         SET year = COALESCE(
               year,
               (SELECT e.year
                FROM events e
                WHERE e.id = albums.event_id
                  AND REPLACE(REPLACE(REPLACE(e.name_norm, ' ', ''), '_', ''), '-', '') <> 'unknownevent')
             ),
             date = CASE
               WHEN date IS NULL OR length(date) <= 4
               THEN COALESCE(
                 (SELECT e.date
                  FROM events e
                  WHERE e.id = albums.event_id
                    AND e.date IS NOT NULL
                    AND REPLACE(REPLACE(REPLACE(e.name_norm, ' ', ''), '_', ''), '-', '') <> 'unknownevent'
                    AND (
                      albums.year IS NULL
                      OR (
                        length(e.date) >= 4
                        AND substr(e.date, 1, 4) GLOB '[0-9][0-9][0-9][0-9]'
                        AND CAST(substr(e.date, 1, 4) AS INTEGER) = albums.year
                      )
                    )),
                 date
               )
               ELSE date
             END
         WHERE event_id IS NOT NULL",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE tracks
         SET year = COALESCE(
               year,
               (SELECT e.year
                FROM events e
                WHERE e.id = tracks.event_id
                  AND REPLACE(REPLACE(REPLACE(e.name_norm, ' ', ''), '_', ''), '-', '') <> 'unknownevent')
             ),
             date = CASE
               WHEN date IS NULL OR length(date) <= 4
               THEN COALESCE(
                 (SELECT e.date
                  FROM events e
                  WHERE e.id = tracks.event_id
                    AND e.date IS NOT NULL
                    AND REPLACE(REPLACE(REPLACE(e.name_norm, ' ', ''), '_', ''), '-', '') <> 'unknownevent'
                    AND (
                      tracks.year IS NULL
                      OR (
                        length(e.date) >= 4
                        AND substr(e.date, 1, 4) GLOB '[0-9][0-9][0-9][0-9]'
                        AND CAST(substr(e.date, 1, 4) AS INTEGER) = tracks.year
                      )
                    )),
                 date
               )
               ELSE date
             END,
             artwork_id = COALESCE(artwork_id, (SELECT artwork_id FROM albums WHERE albums.id = tracks.album_id))
         WHERE album_id IS NOT NULL",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE artists
         SET artwork_id = COALESCE(
           artwork_id,
           (SELECT al.artwork_id
            FROM album_artists aa
            JOIN albums al ON al.id = aa.album_id
            WHERE aa.artist_id = artists.id AND al.artwork_id IS NOT NULL
            ORDER BY COALESCE(al.year, 0) DESC
            LIMIT 1)
         )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    out.push(cur.trim().to_string());
    out
}
