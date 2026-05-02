use anyhow::Result;
use sqlx::{Row, SqlitePool};
use std::collections::HashSet;

pub async fn init_db(pool: &SqlitePool) -> Result<()> {
    let statements = SCHEMA
        .split("\n-- statement\n")
        .map(str::trim)
        .filter(|s| !s.is_empty());
    for statement in statements {
        sqlx::query(statement).execute(pool).await?;
    }
    migrate_db(pool).await?;
    Ok(())
}

async fn migrate_db(pool: &SqlitePool) -> Result<()> {
    ensure_column(pool, "media_files", "sample_rate", "INTEGER").await?;
    ensure_column(pool, "media_files", "channels", "INTEGER").await?;
    ensure_column(pool, "media_files", "duration_ms", "INTEGER").await?;
    ensure_column(pool, "media_files", "scan_error", "TEXT").await?;
    ensure_column(pool, "media_files", "missing", "INTEGER NOT NULL DEFAULT 0").await?;

    ensure_column(pool, "tracks", "date", "TEXT").await?;
    ensure_column(pool, "tracks", "year", "INTEGER").await?;
    ensure_column(pool, "tracks", "artwork_id", "INTEGER").await?;
    ensure_column(pool, "tracks", "liked_at", "INTEGER").await?;

    ensure_column(pool, "albums", "date", "TEXT").await?;
    ensure_column(pool, "albums", "year", "INTEGER").await?;
    ensure_column(pool, "albums", "event_id", "INTEGER").await?;
    ensure_column(pool, "albums", "artwork_id", "INTEGER").await?;
    ensure_column(pool, "albums", "liked_at", "INTEGER").await?;

    ensure_column(pool, "artists", "artwork_id", "INTEGER").await?;
    ensure_column(pool, "artists", "liked_at", "INTEGER").await?;

    ensure_column(pool, "events", "date", "TEXT").await?;
    ensure_column(pool, "events", "year", "INTEGER").await?;
    ensure_column(pool, "events", "liked_at", "INTEGER").await?;

    ensure_column(pool, "cue_sheets", "encoding", "TEXT").await?;
    ensure_column(pool, "cue_sheets", "parse_error", "TEXT").await?;

    ensure_column(
        pool,
        "track_audio_sources",
        "kind",
        "TEXT NOT NULL DEFAULT 'file'",
    )
    .await?;
    ensure_column(pool, "track_audio_sources", "cue_sheet_id", "INTEGER").await?;
    ensure_column(
        pool,
        "track_audio_sources",
        "codec",
        "TEXT NOT NULL DEFAULT ''",
    )
    .await?;
    ensure_column(pool, "track_audio_sources", "sample_rate", "INTEGER").await?;
    ensure_column(pool, "track_audio_sources", "start_sample", "INTEGER").await?;
    ensure_column(pool, "track_audio_sources", "end_sample", "INTEGER").await?;
    ensure_column(pool, "track_audio_sources", "start_ms", "INTEGER").await?;
    ensure_column(pool, "track_audio_sources", "end_ms", "INTEGER").await?;
    ensure_column(
        pool,
        "track_audio_sources",
        "renderer",
        "TEXT NOT NULL DEFAULT 'passthrough'",
    )
    .await?;

    sqlx::query(
        "UPDATE track_audio_sources
         SET renderer = 'ffmpeg_cue'
         WHERE renderer = 'unsupported_cue'",
    )
    .execute(pool)
    .await?;

    reset_legacy_playback_settings(pool).await?;

    Ok(())
}

async fn reset_legacy_playback_settings(pool: &SqlitePool) -> Result<()> {
    let marker = "__app_settings_reset_for_browser_playback_v1";
    let reset_done = sqlx::query("SELECT 1 FROM app_settings WHERE key = ?")
        .bind(marker)
        .fetch_optional(pool)
        .await?
        .is_some();
    if reset_done {
        return Ok(());
    }

    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query("DELETE FROM app_settings")
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO app_settings (key, value_json, updated_at)
         VALUES (?, ?, ?)",
    )
    .bind(marker)
    .bind("true")
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

async fn ensure_column(
    pool: &SqlitePool,
    table: &'static str,
    column: &'static str,
    definition: &'static str,
) -> Result<()> {
    let columns = table_columns(pool, table).await?;
    if columns.contains(column) {
        return Ok(());
    }
    let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {definition}");
    sqlx::query(&sql).execute(pool).await?;
    Ok(())
}

async fn table_columns(pool: &SqlitePool, table: &'static str) -> Result<HashSet<String>> {
    let sql = format!("PRAGMA table_info({table})");
    let rows = sqlx::query(&sql).fetch_all(pool).await?;
    rows.into_iter()
        .map(|row| row.try_get::<String, _>("name").map_err(Into::into))
        .collect()
}

const SCHEMA: &str = r#"
PRAGMA journal_mode = WAL;
-- statement
PRAGMA foreign_keys = ON;
-- statement
CREATE TABLE IF NOT EXISTS media_files (
  id INTEGER PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  path_hash TEXT NOT NULL UNIQUE,
  size INTEGER NOT NULL,
  mtime_ns INTEGER NOT NULL,
  format TEXT NOT NULL,
  sample_rate INTEGER,
  channels INTEGER,
  duration_ms INTEGER,
  last_scanned_at INTEGER NOT NULL,
  scan_error TEXT,
  missing INTEGER NOT NULL DEFAULT 0
);
-- statement
CREATE TABLE IF NOT EXISTS events (
  id INTEGER PRIMARY KEY,
  uuid TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  name_norm TEXT NOT NULL UNIQUE,
  date TEXT,
  year INTEGER,
  liked_at INTEGER
);
-- statement
CREATE TABLE IF NOT EXISTS albums (
  id INTEGER PRIMARY KEY,
  uuid TEXT NOT NULL UNIQUE,
  title TEXT NOT NULL,
  title_norm TEXT NOT NULL,
  date TEXT,
  year INTEGER,
  event_id INTEGER,
  artwork_id INTEGER,
  liked_at INTEGER,
  FOREIGN KEY(event_id) REFERENCES events(id),
  FOREIGN KEY(artwork_id) REFERENCES artwork_sources(id)
);
-- statement
CREATE TABLE IF NOT EXISTS artists (
  id INTEGER PRIMARY KEY,
  uuid TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  name_norm TEXT NOT NULL UNIQUE,
  artwork_id INTEGER,
  liked_at INTEGER,
  FOREIGN KEY(artwork_id) REFERENCES artwork_sources(id)
);
-- statement
CREATE TABLE IF NOT EXISTS tracks (
  id INTEGER PRIMARY KEY,
  uuid TEXT NOT NULL UNIQUE,
  title TEXT NOT NULL,
  title_norm TEXT NOT NULL,
  album_id INTEGER,
  event_id INTEGER,
  cue_track_no INTEGER,
  disc_no INTEGER,
  track_no INTEGER,
  duration_ms INTEGER,
  date TEXT,
  year INTEGER,
  artwork_id INTEGER,
  liked_at INTEGER,
  FOREIGN KEY(album_id) REFERENCES albums(id) ON DELETE SET NULL,
  FOREIGN KEY(event_id) REFERENCES events(id) ON DELETE SET NULL,
  FOREIGN KEY(artwork_id) REFERENCES artwork_sources(id)
);
-- statement
CREATE TABLE IF NOT EXISTS cue_sheets (
  id INTEGER PRIMARY KEY,
  cue_file_id INTEGER NOT NULL UNIQUE,
  audio_file_id INTEGER NOT NULL,
  album_title TEXT,
  performer TEXT,
  date TEXT,
  encoding TEXT,
  parse_error TEXT,
  FOREIGN KEY(cue_file_id) REFERENCES media_files(id) ON DELETE CASCADE,
  FOREIGN KEY(audio_file_id) REFERENCES media_files(id) ON DELETE CASCADE
);
-- statement
CREATE TABLE IF NOT EXISTS track_audio_sources (
  id INTEGER PRIMARY KEY,
  track_id INTEGER NOT NULL UNIQUE,
  kind TEXT NOT NULL,
  media_file_id INTEGER NOT NULL,
  cue_sheet_id INTEGER,
  codec TEXT NOT NULL,
  sample_rate INTEGER,
  start_sample INTEGER,
  end_sample INTEGER,
  start_ms INTEGER,
  end_ms INTEGER,
  renderer TEXT NOT NULL,
  FOREIGN KEY(track_id) REFERENCES tracks(id) ON DELETE CASCADE,
  FOREIGN KEY(media_file_id) REFERENCES media_files(id) ON DELETE CASCADE,
  FOREIGN KEY(cue_sheet_id) REFERENCES cue_sheets(id) ON DELETE CASCADE
);
-- statement
CREATE TABLE IF NOT EXISTS track_artists (
  track_id INTEGER NOT NULL,
  artist_id INTEGER NOT NULL,
  role TEXT NOT NULL DEFAULT 'main',
  position INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(track_id, artist_id, role),
  FOREIGN KEY(track_id) REFERENCES tracks(id) ON DELETE CASCADE,
  FOREIGN KEY(artist_id) REFERENCES artists(id) ON DELETE CASCADE
);
-- statement
CREATE TABLE IF NOT EXISTS album_artists (
  album_id INTEGER NOT NULL,
  artist_id INTEGER NOT NULL,
  position INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(album_id, artist_id),
  FOREIGN KEY(album_id) REFERENCES albums(id) ON DELETE CASCADE,
  FOREIGN KEY(artist_id) REFERENCES artists(id) ON DELETE CASCADE
);
-- statement
CREATE TABLE IF NOT EXISTS event_albums (
  event_id INTEGER NOT NULL,
  album_id INTEGER NOT NULL,
  position INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(event_id, album_id),
  FOREIGN KEY(event_id) REFERENCES events(id) ON DELETE CASCADE,
  FOREIGN KEY(album_id) REFERENCES albums(id) ON DELETE CASCADE
);
-- statement
CREATE TABLE IF NOT EXISTS artist_aliases (
  id INTEGER PRIMARY KEY,
  artist_id INTEGER NOT NULL,
  alias TEXT NOT NULL,
  alias_norm TEXT NOT NULL UNIQUE,
  FOREIGN KEY(artist_id) REFERENCES artists(id) ON DELETE CASCADE
);
-- statement
CREATE TABLE IF NOT EXISTS artist_split_exceptions (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  name_norm TEXT NOT NULL UNIQUE,
  created_at INTEGER NOT NULL
);
-- statement
CREATE TABLE IF NOT EXISTS artist_merge_audit (
  id INTEGER PRIMARY KEY,
  target_artist_id INTEGER NOT NULL,
  source_artist_uuid TEXT NOT NULL,
  source_artist_name TEXT NOT NULL,
  reason TEXT NOT NULL,
  merged_at INTEGER NOT NULL,
  FOREIGN KEY(target_artist_id) REFERENCES artists(id)
);
-- statement
CREATE TABLE IF NOT EXISTS artist_relation_edges (
  artist_a_id INTEGER NOT NULL,
  artist_b_id INTEGER NOT NULL,
  strength INTEGER NOT NULL,
  details_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY(artist_a_id, artist_b_id),
  FOREIGN KEY(artist_a_id) REFERENCES artists(id) ON DELETE CASCADE,
  FOREIGN KEY(artist_b_id) REFERENCES artists(id) ON DELETE CASCADE
);
-- statement
CREATE TABLE IF NOT EXISTS lyric_cache (
  id INTEGER PRIMARY KEY,
  track_id INTEGER,
  title TEXT NOT NULL,
  artist TEXT NOT NULL,
  album TEXT,
  duration_ms INTEGER,
  provider TEXT NOT NULL,
  lyrics TEXT NOT NULL,
  score REAL,
  created_at INTEGER NOT NULL,
  FOREIGN KEY(track_id) REFERENCES tracks(id) ON DELETE SET NULL
);
-- statement
CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);
-- statement
CREATE TABLE IF NOT EXISTS scan_jobs (
  id INTEGER PRIMARY KEY,
  status TEXT NOT NULL,
  root_paths TEXT NOT NULL,
  total_files INTEGER NOT NULL DEFAULT 0,
  scanned_files INTEGER NOT NULL DEFAULT 0,
  started_at INTEGER NOT NULL,
  finished_at INTEGER,
  error TEXT
);
-- statement
CREATE TABLE IF NOT EXISTS artwork_sources (
  id INTEGER PRIMARY KEY,
  kind TEXT NOT NULL,
  media_file_id INTEGER,
  sidecar_path TEXT,
  embedded_picture_index INTEGER,
  mime TEXT,
  width INTEGER,
  height INTEGER,
  content_hash TEXT,
  last_checked_at INTEGER,
  FOREIGN KEY(media_file_id) REFERENCES media_files(id) ON DELETE CASCADE
);
-- statement
CREATE TABLE IF NOT EXISTS artwork_blobs (
  id INTEGER PRIMARY KEY,
  source_id INTEGER NOT NULL,
  variant TEXT NOT NULL,
  mime TEXT NOT NULL,
  width INTEGER,
  height INTEGER,
  bytes BLOB NOT NULL,
  created_at INTEGER NOT NULL,
  UNIQUE(source_id, variant),
  FOREIGN KEY(source_id) REFERENCES artwork_sources(id) ON DELETE CASCADE
);
-- statement
CREATE VIRTUAL TABLE IF NOT EXISTS search_index USING fts5(
  kind,
  entity_id UNINDEXED,
  title,
  artists,
  album,
  event,
  aliases,
  tokenize = 'unicode61'
);
-- statement
CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks(album_id, disc_no, track_no);
-- statement
CREATE INDEX IF NOT EXISTS idx_track_audio_sources_media ON track_audio_sources(media_file_id);
-- statement
CREATE INDEX IF NOT EXISTS idx_track_artists_artist ON track_artists(artist_id, track_id);
-- statement
CREATE INDEX IF NOT EXISTS idx_album_artists_artist ON album_artists(artist_id, album_id);
-- statement
CREATE INDEX IF NOT EXISTS idx_events_name_norm ON events(name_norm);
-- statement
CREATE INDEX IF NOT EXISTS idx_events_year_date ON events(year, date);
-- statement
CREATE INDEX IF NOT EXISTS idx_albums_title_norm ON albums(title_norm);
-- statement
CREATE INDEX IF NOT EXISTS idx_tracks_title_norm ON tracks(title_norm);
-- statement
CREATE INDEX IF NOT EXISTS idx_tracks_event ON tracks(event_id, album_id);
-- statement
CREATE INDEX IF NOT EXISTS idx_event_albums_album ON event_albums(album_id, event_id);
-- statement
CREATE INDEX IF NOT EXISTS idx_artist_relations_b ON artist_relation_edges(artist_b_id);
-- statement
CREATE INDEX IF NOT EXISTS idx_media_files_scan ON media_files(path_hash, size, mtime_ns);
"#;
