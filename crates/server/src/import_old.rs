use crate::db;
use anyhow::{Context, Result};
use easy_musiclib_media::formats::{PASSTHROUGH_RENDERER, format_by_extension};
use easy_musiclib_media::normalize::normalize_name;
use easy_musiclib_media::path_hash;
use serde_json::Value;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::path::Path;

pub async fn import_library_data(
    pool: &SqlitePool,
    path: &Path,
    stat_files: bool,
) -> Result<ImportReport> {
    eprintln!("import-json: reading {}", path.display());
    sqlx::query("BEGIN IMMEDIATE").execute(pool).await?;
    let result = import_library_data_inner(pool, path, stat_files).await;
    match result {
        Ok(report) => {
            sqlx::query("COMMIT").execute(pool).await?;
            eprintln!("import-json: complete");
            Ok(report)
        }
        Err(err) => {
            let _ = sqlx::query("ROLLBACK").execute(pool).await;
            Err(err)
        }
    }
}

async fn import_library_data_inner(
    pool: &SqlitePool,
    path: &Path,
    stat_files: bool,
) -> Result<ImportReport> {
    let data: Value = serde_json::from_slice(
        &std::fs::read(path).with_context(|| format!("reading {}", path.display()))?,
    )?;
    let mut report = ImportReport::default();
    let mut artist_ids = HashMap::new();
    let mut event_ids = HashMap::new();
    let mut album_ids = HashMap::new();

    if let Some(events) = data.get("events").and_then(|v| v.as_object()) {
        eprintln!("import-json: importing {} events", events.len());
        for (uuid, event) in events {
            let id = insert_event(pool, uuid, event).await?;
            event_ids.insert(uuid.clone(), id);
            report.events += 1;
            log_progress("events", report.events, events.len());
        }
    }

    if let Some(artists) = data.get("artists").and_then(|v| v.as_object()) {
        eprintln!("import-json: importing {} artists", artists.len());
        for (uuid, artist) in artists {
            let id = insert_artist(pool, uuid, artist).await?;
            artist_ids.insert(uuid.clone(), id);
            report.artists += 1;
            log_progress("artists", report.artists, artists.len());
        }
    }

    if let Some(albums) = data.get("albums").and_then(|v| v.as_object()) {
        eprintln!("import-json: importing {} albums", albums.len());
        for (uuid, album) in albums {
            let event_id = album
                .get("event")
                .and_then(|e| e.get("uuid"))
                .and_then(|v| v.as_str())
                .and_then(|uuid| event_ids.get(uuid).copied());
            let artwork_id = import_artwork(pool, album.get("album_art_path")).await?;
            let id = insert_album(pool, uuid, album, event_id, artwork_id).await?;
            if let Some(album_artists) = album.get("album_artists").and_then(|v| v.as_array()) {
                for (pos, artist_uuid) in
                    album_artists.iter().filter_map(|v| v.as_str()).enumerate()
                {
                    if let Some(artist_id) = artist_ids.get(artist_uuid) {
                        sqlx::query(
                            "INSERT OR IGNORE INTO album_artists (album_id, artist_id, position) VALUES (?, ?, ?)",
                        )
                        .bind(id)
                        .bind(artist_id)
                        .bind(pos as i64)
                        .execute(pool)
                        .await?;
                    }
                }
            }
            if let Some(event_id) = event_id {
                sqlx::query(
                    "INSERT OR IGNORE INTO event_albums (event_id, album_id) VALUES (?, ?)",
                )
                .bind(event_id)
                .bind(id)
                .execute(pool)
                .await?;
            }
            album_ids.insert(uuid.clone(), id);
            report.albums += 1;
            log_progress("albums", report.albums, albums.len());
        }
    }

    if let Some(songs) = data.get("songs").and_then(|v| v.as_object()) {
        eprintln!(
            "import-json: importing {} tracks{}",
            songs.len(),
            if stat_files {
                " with file metadata checks"
            } else {
                ""
            }
        );
        for (uuid, song) in songs {
            let album_id = song
                .get("album")
                .and_then(|v| v.as_str())
                .and_then(|uuid| album_ids.get(uuid).copied());
            let event_id = song
                .get("event")
                .and_then(|e| e.get("uuid"))
                .and_then(|v| v.as_str())
                .and_then(|uuid| event_ids.get(uuid).copied());
            let artwork_id = import_artwork(pool, song.get("song_art_path")).await?;
            let track_id = insert_song(pool, uuid, song, album_id, event_id, artwork_id).await?;
            if let Some(artists) = song.get("artists").and_then(|v| v.as_array()) {
                for (pos, artist_uuid) in artists.iter().filter_map(|v| v.as_str()).enumerate() {
                    if let Some(artist_id) = artist_ids.get(artist_uuid) {
                        sqlx::query(
                            "INSERT OR IGNORE INTO track_artists (track_id, artist_id, position) VALUES (?, ?, ?)",
                        )
                        .bind(track_id)
                        .bind(artist_id)
                        .bind(pos as i64)
                        .execute(pool)
                        .await?;
                    }
                }
            }
            import_audio_source(pool, track_id, song, stat_files).await?;
            report.tracks += 1;
            log_progress("tracks", report.tracks, songs.len());
        }
    }

    eprintln!("import-json: rebuilding search index");
    rebuild_search_index(pool).await?;
    eprintln!("import-json: repairing inherited dates and artwork");
    db::repair_event_dates_and_artwork(pool).await?;
    eprintln!("import-json: rebuilding relation graph");
    db::rebuild_relations(pool).await?;
    Ok(report)
}

#[derive(Debug, Clone, Default)]
pub struct ImportReport {
    pub artists: usize,
    pub albums: usize,
    pub tracks: usize,
    pub events: usize,
}

async fn insert_artist(pool: &SqlitePool, uuid: &str, artist: &Value) -> Result<i64> {
    let name = text(artist, "name").unwrap_or("Unknown Artist");
    let artwork_id = import_artwork(pool, artist.get("artist_art_path")).await?;
    let liked_at = liked_at(artist);
    if let Some(row) = sqlx::query("SELECT id FROM artists WHERE uuid = ?")
        .bind(uuid)
        .fetch_optional(pool)
        .await?
    {
        return Ok(row.try_get("id")?);
    }
    let res = sqlx::query(
        "INSERT INTO artists (uuid, name, name_norm, artwork_id, liked_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(uuid)
    .bind(name)
    .bind(normalize_name(name, false))
    .bind(artwork_id)
    .bind(liked_at)
    .execute(pool)
    .await?;
    Ok(res.last_insert_rowid())
}

async fn insert_event(pool: &SqlitePool, uuid: &str, event: &Value) -> Result<i64> {
    let name = text(event, "name").unwrap_or("Unknown Event");
    if let Some(row) = sqlx::query("SELECT id FROM events WHERE uuid = ?")
        .bind(uuid)
        .fetch_optional(pool)
        .await?
    {
        return Ok(row.try_get("id")?);
    }
    let res = sqlx::query(
        "INSERT INTO events (uuid, name, name_norm, date, year, liked_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(uuid)
    .bind(name)
    .bind(normalize_name(name, false))
    .bind(text(event, "date"))
    .bind(int(event, "year"))
    .bind(liked_at(event))
    .execute(pool)
    .await?;
    Ok(res.last_insert_rowid())
}

async fn insert_album(
    pool: &SqlitePool,
    uuid: &str,
    album: &Value,
    event_id: Option<i64>,
    artwork_id: Option<i64>,
) -> Result<i64> {
    let title = text(album, "name").unwrap_or("Unknown Album");
    if let Some(row) = sqlx::query("SELECT id FROM albums WHERE uuid = ?")
        .bind(uuid)
        .fetch_optional(pool)
        .await?
    {
        return Ok(row.try_get("id")?);
    }
    let res = sqlx::query(
        "INSERT INTO albums (uuid, title, title_norm, date, year, event_id, artwork_id, liked_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(uuid)
    .bind(title)
    .bind(normalize_name(title, false))
    .bind(text(album, "date"))
    .bind(int(album, "year"))
    .bind(event_id)
    .bind(artwork_id)
    .bind(liked_at(album))
    .execute(pool)
    .await?;
    Ok(res.last_insert_rowid())
}

async fn insert_song(
    pool: &SqlitePool,
    uuid: &str,
    song: &Value,
    album_id: Option<i64>,
    event_id: Option<i64>,
    artwork_id: Option<i64>,
) -> Result<i64> {
    let title = text(song, "name").unwrap_or("Unknown Title");
    if let Some(row) = sqlx::query("SELECT id FROM tracks WHERE uuid = ?")
        .bind(uuid)
        .fetch_optional(pool)
        .await?
    {
        return Ok(row.try_get("id")?);
    }
    let res = sqlx::query(
        "INSERT INTO tracks
         (uuid, title, title_norm, album_id, event_id, disc_no, track_no, date, year, artwork_id, liked_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(uuid)
    .bind(title)
    .bind(normalize_name(title, false))
    .bind(album_id)
    .bind(event_id)
    .bind(int(song, "disc_number"))
    .bind(int(song, "track_number"))
    .bind(text(song, "date"))
    .bind(int(song, "year"))
    .bind(artwork_id)
    .bind(liked_at(song))
    .execute(pool)
    .await?;
    Ok(res.last_insert_rowid())
}

async fn import_audio_source(
    pool: &SqlitePool,
    track_id: i64,
    song: &Value,
    stat_files: bool,
) -> Result<()> {
    let Some(path) = text(song, "file_path").filter(|p| !p.trim().is_empty()) else {
        return Ok(());
    };
    let path_obj = Path::new(path);
    let format = format_by_extension(path_obj)
        .map(|format| format.id())
        .unwrap_or("unknown");
    let (size, mtime_ns) = if stat_files {
        std::fs::metadata(path_obj)
            .map(|meta| {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    (
                        meta.len().try_into().unwrap_or(i64::MAX),
                        meta.mtime()
                            .saturating_mul(1_000_000_000)
                            .saturating_add(meta.mtime_nsec()),
                    )
                }
                #[cfg(not(unix))]
                {
                    (meta.len().try_into().unwrap_or(i64::MAX), 0)
                }
            })
            .unwrap_or((0, 0))
    } else {
        (0, 0)
    };
    let (media_id, _) =
        db::upsert_media_file(pool, path, &path_hash(path_obj), size, mtime_ns, format).await?;
    sqlx::query(
        "INSERT OR IGNORE INTO track_audio_sources
         (track_id, kind, media_file_id, codec, renderer)
         VALUES (?, 'file', ?, ?, ?)",
    )
    .bind(track_id)
    .bind(media_id)
    .bind(format)
    .bind(PASSTHROUGH_RENDERER)
    .execute(pool)
    .await?;
    Ok(())
}

async fn rebuild_search_index(pool: &SqlitePool) -> Result<()> {
    sqlx::query("DELETE FROM search_index")
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO search_index (kind, entity_id, title, artists, album, event, aliases)
         SELECT
           'track',
           t.id,
           t.title,
           COALESCE((
             SELECT group_concat(name, ' ')
             FROM (
               SELECT a.name AS name
               FROM track_artists ta
               JOIN artists a ON a.id = ta.artist_id
               WHERE ta.track_id = t.id
               ORDER BY ta.position, a.name
             )
           ), ''),
           al.title,
           ev.name,
           ''
         FROM tracks t
         LEFT JOIN albums al ON al.id = t.album_id
         LEFT JOIN events ev ON ev.id = t.event_id",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO search_index (kind, entity_id, title, artists, album, event, aliases)
         SELECT
           'album',
           al.id,
           al.title,
           COALESCE((
             SELECT group_concat(name, ' ')
             FROM (
               SELECT a.name AS name
               FROM album_artists aa
               JOIN artists a ON a.id = aa.artist_id
               WHERE aa.album_id = al.id
               ORDER BY aa.position, a.name
             )
           ), ''),
           '',
           ev.name,
           ''
         FROM albums al
         LEFT JOIN events ev ON ev.id = al.event_id",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO search_index (kind, entity_id, title, artists, album, event, aliases)
         SELECT
           'artist',
           a.id,
           a.name,
           '',
           '',
           '',
           COALESCE((
             SELECT group_concat(alias, ' ')
             FROM (
               SELECT alias
               FROM artist_aliases
               WHERE artist_id = a.id
               ORDER BY alias
             )
           ), '')
         FROM artists a",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO search_index (kind, entity_id, title, artists, album, event, aliases)
         SELECT 'event', id, name, '', '', name, ''
         FROM events",
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn import_artwork(pool: &SqlitePool, value: Option<&Value>) -> Result<Option<i64>> {
    let Some(path) = value
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return Ok(None);
    };
    db::ensure_artwork_source(pool, "sidecar", None, Some(path), None, None).await
}

fn log_progress(kind: &str, done: usize, total: usize) {
    if done == total || done % 1000 == 0 {
        eprintln!("import-json: {kind}: {done}/{total}");
    }
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(|v| v.as_str())
}

fn int(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(|v| v.as_i64())
}

fn liked_at(value: &Value) -> Option<i64> {
    if !value
        .get("is_liked")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return None;
    }
    value
        .get("liked_time")
        .and_then(|v| v.as_str())
        .and_then(parse_time_ms)
        .or_else(|| Some(db::now_ms()))
}

fn parse_time_ms(input: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(input)
        .map(|dt| dt.timestamp_millis())
        .ok()
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%S%.f")
                .ok()
                .map(|dt| dt.and_utc().timestamp_millis())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn imports_legacy_json_and_builds_indexes() -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        schema::init_db(&pool).await?;

        let temp = tempfile::tempdir()?;
        let json_path = temp.path().join("library_data.json");
        std::fs::write(
            &json_path,
            r#"{
              "events": {
                "event-1": {"name": "M3", "date": "2024-04-28", "year": 2024}
              },
              "artists": {
                "artist-1": {"name": "Alice", "is_liked": true}
              },
              "albums": {
                "album-1": {
                  "name": "Demo Album",
                  "event": {"uuid": "event-1"},
                  "album_artists": ["artist-1"]
                }
              },
              "songs": {
                "song-1": {
                  "name": "Demo Track",
                  "album": "album-1",
                  "event": {"uuid": "event-1"},
                  "artists": ["artist-1"],
                  "file_path": "/music/demo.flac",
                  "track_number": 1,
                  "disc_number": 1,
                  "is_liked": true,
                  "liked_time": "2024-01-02T03:04:05Z"
                }
              }
            }"#,
        )?;

        let report = import_library_data(&pool, &json_path, false).await?;
        assert_eq!(report.events, 1);
        assert_eq!(report.artists, 1);
        assert_eq!(report.albums, 1);
        assert_eq!(report.tracks, 1);

        let track_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracks")
            .fetch_one(&pool)
            .await?;
        assert_eq!(track_count, 1);
        let source_path: String =
            sqlx::query_scalar("SELECT path FROM media_files WHERE path = '/music/demo.flac'")
                .fetch_one(&pool)
                .await?;
        assert_eq!(source_path, "/music/demo.flac");
        let search_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM search_index WHERE title MATCH 'Demo'")
                .fetch_one(&pool)
                .await?;
        assert!(search_count >= 2);
        Ok(())
    }
}
