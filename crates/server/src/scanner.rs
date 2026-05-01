use crate::db;
use anyhow::{Context, Result};
use easy_musiclib_media::cue::{self, apply_audio_timing, cue_year, parse_cue_file};
use easy_musiclib_media::formats::{
    PASSTHROUGH_RENDERER, cue_renderer_id_for_format_id, format_by_extension, read_audio_metadata,
};
use easy_musiclib_media::path_hash;
use easy_musiclib_media::providers::{
    DiscoveredAudioFile, DiscoveredCueFile, discover_library_files,
};
use easy_musiclib_media::tags::AudioTags;
use sqlx::{Row, SqlitePool};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn spawn_scan(pool: SqlitePool, job_id: i64, roots: Vec<String>) {
    tokio::spawn(async move {
        if let Err(err) = run_scan(&pool, job_id, roots).await {
            tracing::error!(job_id, error = %err, "scan failed");
            let _ = db::update_scan_job_counts(
                &pool,
                job_id,
                "failed",
                None,
                None,
                Some(&err.to_string()),
                true,
            )
            .await;
        }
    });
}

pub async fn run_scan(pool: &SqlitePool, job_id: i64, roots: Vec<String>) -> Result<()> {
    db::update_scan_job_counts(pool, job_id, "running", None, Some(0), None, false).await?;
    ensure_default_split_exceptions(pool).await?;

    let discovered = tokio::task::spawn_blocking({
        let roots = roots.clone();
        move || discover_library_files(&roots)
    })
    .await??;
    db::update_scan_job_counts(
        pool,
        job_id,
        "running",
        Some(discovered.len() as i64),
        Some(0),
        None,
        false,
    )
    .await?;

    let mut cue_audio_paths = HashSet::new();
    for file in &discovered.cues {
        if let Ok(sheet) = parse_cue_file(&file.path) {
            cue_audio_paths.insert(normalize_path_key(&sheet.audio_path));
        }
    }

    let exceptions = split_exceptions(pool).await?;
    let mut scanned = 0i64;

    for file in &discovered.cues {
        if let Err(err) = process_cue(pool, file, &exceptions).await {
            tracing::error!(path = %file.path.display(), error = %err, "failed to process cue file");
        }
        scanned += 1;
        db::update_scan_job_counts(pool, job_id, "running", None, Some(scanned), None, false)
            .await?;
    }

    for file in &discovered.audio {
        if cue_audio_paths.contains(&normalize_path_key(&file.path)) {
            scanned += 1;
            db::update_scan_job_counts(pool, job_id, "running", None, Some(scanned), None, false)
                .await?;
            continue;
        }
        if let Err(err) = process_audio_file(pool, file, &exceptions).await {
            tracing::error!(path = %file.path.display(), error = %err, "failed to process audio file");
        }
        scanned += 1;
        db::update_scan_job_counts(pool, job_id, "running", None, Some(scanned), None, false)
            .await?;
    }

    db::repair_event_dates_and_artwork(pool).await?;
    db::rebuild_relations(pool).await?;
    db::auto_merge(pool).await.ok();
    db::update_scan_job_counts(pool, job_id, "completed", None, Some(scanned), None, true).await?;
    Ok(())
}

async fn process_audio_file(
    pool: &SqlitePool,
    file: &DiscoveredAudioFile,
    exceptions: &[String],
) -> Result<()> {
    let path_string = file.path.to_string_lossy().to_string();
    let (media_file_id, changed) = db::upsert_media_file(
        pool,
        &path_string,
        &file.path_hash,
        file.size,
        file.mtime_ns,
        &file.format,
    )
    .await?;
    if !changed && has_sources_for_media(pool, media_file_id).await? {
        return Ok(());
    }
    db::delete_tracks_for_media_file(pool, media_file_id).await?;
    match read_audio_metadata(&file.path, exceptions) {
        Ok(tags) => {
            sqlx::query(
                "UPDATE media_files SET scan_error = NULL, sample_rate = ?, channels = ?, duration_ms = ? WHERE id = ?",
            )
            .bind(tags.sample_rate)
            .bind(tags.channels)
            .bind(tags.duration_ms)
            .bind(media_file_id)
            .execute(pool)
            .await?;
            write_audio_track(pool, media_file_id, &file.format, tags).await?;
        }
        Err(err) => {
            sqlx::query("UPDATE media_files SET scan_error = ? WHERE id = ?")
                .bind(err.to_string())
                .bind(media_file_id)
                .execute(pool)
                .await?;
        }
    }
    Ok(())
}

async fn write_audio_track(
    pool: &SqlitePool,
    media_file_id: i64,
    format: &str,
    tags: AudioTags,
) -> Result<i64> {
    let artwork_id = artwork_for_tags(pool, media_file_id, &tags).await?;
    let artist_ids = ensure_artists(pool, &tags.artists, artwork_id).await?;
    let album_artist_ids = ensure_artists(pool, &tags.album_artists, artwork_id).await?;
    let event_id =
        db::ensure_event(pool, tags.event.as_deref(), tags.date.as_deref(), tags.year).await?;
    let album_id = db::find_or_create_album(
        pool,
        &tags.album,
        &album_artist_ids,
        tags.year,
        tags.date.as_deref(),
        event_id,
        artwork_id,
    )
    .await?;
    if let Some(event_id) = event_id {
        sqlx::query("INSERT OR IGNORE INTO event_albums (event_id, album_id) VALUES (?, ?)")
            .bind(event_id)
            .bind(album_id)
            .execute(pool)
            .await?;
    }
    let track_id = db::insert_track(
        pool,
        db::NewTrack {
            title: &tags.title,
            album_id: Some(album_id),
            event_id,
            cue_track_no: None,
            disc_no: tags.disc_number,
            track_no: tags.track_number,
            duration_ms: tags.duration_ms,
            date: tags.date.as_deref(),
            year: tags.year,
            artwork_id,
        },
        &artist_ids,
    )
    .await?;
    db::insert_track_audio_source(
        pool,
        track_id,
        db::NewTrackAudioSource {
            kind: "file",
            media_file_id,
            cue_sheet_id: None,
            codec: format,
            sample_rate: tags.sample_rate,
            start_sample: None,
            end_sample: None,
            start_ms: None,
            end_ms: None,
            renderer: PASSTHROUGH_RENDERER,
        },
    )
    .await?;
    db::refresh_track_search(pool, track_id).await?;
    db::refresh_album_search(pool, album_id).await?;
    for artist_id in artist_ids.iter().chain(album_artist_ids.iter()) {
        db::refresh_artist_search(pool, *artist_id).await?;
    }
    if let Some(event_id) = event_id {
        db::refresh_event_search(pool, event_id).await?;
    }
    Ok(track_id)
}

async fn process_cue(
    pool: &SqlitePool,
    file: &DiscoveredCueFile,
    exceptions: &[String],
) -> Result<()> {
    let path_string = file.path.to_string_lossy().to_string();
    let (cue_file_id, cue_changed) = db::upsert_media_file(
        pool,
        &path_string,
        &file.path_hash,
        file.size,
        file.mtime_ns,
        cue::FORMAT_ID,
    )
    .await?;

    let mut sheet = match parse_cue_file(&file.path) {
        Ok(sheet) => sheet,
        Err(err) => {
            sqlx::query("UPDATE media_files SET scan_error = ? WHERE id = ?")
                .bind(err.to_string())
                .bind(cue_file_id)
                .execute(pool)
                .await?;
            return Ok(());
        }
    };
    let audio_meta = file_meta(&sheet.audio_path)?;
    let audio_format = format_by_extension(&sheet.audio_path)
        .map(|format| format.id())
        .unwrap_or("unknown");
    let audio_path = sheet.audio_path.to_string_lossy().to_string();
    let (audio_file_id, audio_changed) = db::upsert_media_file(
        pool,
        &audio_path,
        &path_hash(&sheet.audio_path),
        audio_meta.0,
        audio_meta.1,
        audio_format,
    )
    .await?;
    let already_has_sheet = sqlx::query("SELECT id FROM cue_sheets WHERE cue_file_id = ?")
        .bind(cue_file_id)
        .fetch_optional(pool)
        .await?
        .is_some();
    if !cue_changed && !audio_changed && already_has_sheet {
        return Ok(());
    }
    db::delete_cue_sheet_for_file(pool, cue_file_id).await?;

    let audio_tags =
        read_audio_metadata(&sheet.audio_path, exceptions).unwrap_or_else(|_| AudioTags {
            title: sheet
                .album_title
                .clone()
                .unwrap_or_else(|| "Unknown Title".to_string()),
            album: sheet
                .album_title
                .clone()
                .unwrap_or_else(|| "Unknown Album".to_string()),
            artists: sheet
                .performer
                .clone()
                .map(|s| vec![s])
                .unwrap_or_else(|| vec!["Unknown Artist".to_string()]),
            album_artists: sheet
                .performer
                .clone()
                .map(|s| vec![s])
                .unwrap_or_else(|| vec!["Unknown Artist".to_string()]),
            raw_artists: Vec::new(),
            raw_album_artists: Vec::new(),
            track_number: None,
            disc_number: Some(1),
            date: sheet.date.clone(),
            year: cue_year(&sheet),
            event: None,
            duration_ms: None,
            sample_rate: None,
            channels: None,
            embedded_picture: None,
            sidecar_artwork: None,
            format: audio_format.to_string(),
        });
    sqlx::query(
        "UPDATE media_files SET scan_error = NULL, sample_rate = ?, channels = ?, duration_ms = ? WHERE id = ?",
    )
    .bind(audio_tags.sample_rate)
    .bind(audio_tags.channels)
    .bind(audio_tags.duration_ms)
    .bind(audio_file_id)
    .execute(pool)
    .await?;
    apply_audio_timing(&mut sheet, audio_tags.sample_rate, audio_tags.duration_ms);
    let cue_sheet_id = db::insert_cue_sheet(
        pool,
        cue_file_id,
        audio_file_id,
        sheet.album_title.as_deref(),
        sheet.performer.as_deref(),
        sheet.date.as_deref(),
    )
    .await?;

    let artwork_id = artwork_for_tags(pool, audio_file_id, &audio_tags).await?;
    let album_name = sheet
        .album_title
        .as_deref()
        .unwrap_or(audio_tags.album.as_str());
    let album_artist_names = sheet
        .performer
        .as_ref()
        .map(|s| vec![s.clone()])
        .unwrap_or_else(|| audio_tags.album_artists.clone());
    let album_artist_names =
        easy_musiclib_media::artists::parse_artists(&album_artist_names, exceptions);
    let album_artist_ids = ensure_artists(pool, &album_artist_names, artwork_id).await?;
    let date = sheet.date.as_deref().or(audio_tags.date.as_deref());
    let year = cue_year(&sheet).or(audio_tags.year);
    let event_id = db::ensure_event(pool, audio_tags.event.as_deref(), date, year).await?;
    let album_id = db::find_or_create_album(
        pool,
        album_name,
        &album_artist_ids,
        year,
        date,
        event_id,
        artwork_id,
    )
    .await?;
    if let Some(event_id) = event_id {
        sqlx::query("INSERT OR IGNORE INTO event_albums (event_id, album_id) VALUES (?, ?)")
            .bind(event_id)
            .bind(album_id)
            .execute(pool)
            .await?;
    }

    for cue_track in &sheet.tracks {
        let artist_names = cue_track
            .performer
            .as_ref()
            .map(|s| vec![s.clone()])
            .unwrap_or_else(|| audio_tags.artists.clone());
        let artist_names = easy_musiclib_media::artists::parse_artists(&artist_names, exceptions);
        let artist_ids = ensure_artists(pool, &artist_names, artwork_id).await?;
        let title = cue_track
            .title
            .as_deref()
            .unwrap_or_else(|| audio_tags.title.as_str());
        let duration_ms = cue_track
            .end_ms
            .map(|end| end.saturating_sub(cue_track.start_ms));
        let track_id = db::insert_track(
            pool,
            db::NewTrack {
                title,
                album_id: Some(album_id),
                event_id,
                cue_track_no: Some(cue_track.no),
                disc_no: audio_tags.disc_number.or(Some(1)),
                track_no: Some(cue_track.no),
                duration_ms,
                date,
                year,
                artwork_id,
            },
            &artist_ids,
        )
        .await?;
        let renderer = cue_renderer_id_for_format_id(audio_format);
        db::insert_track_audio_source(
            pool,
            track_id,
            db::NewTrackAudioSource {
                kind: "cue",
                media_file_id: audio_file_id,
                cue_sheet_id: Some(cue_sheet_id),
                codec: audio_format,
                sample_rate: audio_tags.sample_rate,
                start_sample: cue_track.start_sample,
                end_sample: cue_track.end_sample,
                start_ms: Some(cue_track.start_ms),
                end_ms: cue_track.end_ms,
                renderer,
            },
        )
        .await?;
        db::refresh_track_search(pool, track_id).await?;
        for artist_id in artist_ids {
            db::refresh_artist_search(pool, artist_id).await?;
        }
    }
    db::refresh_album_search(pool, album_id).await?;
    for artist_id in album_artist_ids {
        db::refresh_artist_search(pool, artist_id).await?;
    }
    if let Some(event_id) = event_id {
        db::refresh_event_search(pool, event_id).await?;
    }
    Ok(())
}

async fn ensure_artists(
    pool: &SqlitePool,
    names: &[String],
    artwork_id: Option<i64>,
) -> Result<Vec<i64>> {
    let mut out = Vec::new();
    for name in names.iter().filter(|name| !name.trim().is_empty()) {
        let id = db::ensure_artist(pool, name, artwork_id).await?;
        out.push(id);
    }
    if out.is_empty() {
        out.push(db::ensure_artist(pool, "Unknown Artist", artwork_id).await?);
    }
    Ok(out)
}

async fn artwork_for_tags(
    pool: &SqlitePool,
    media_file_id: i64,
    tags: &AudioTags,
) -> Result<Option<i64>> {
    if let Some(pic) = &tags.embedded_picture {
        return db::ensure_artwork_source(
            pool,
            "embedded",
            Some(media_file_id),
            None,
            Some(pic.index),
            pic.mime.as_deref(),
        )
        .await;
    }
    if let Some(path) = &tags.sidecar_artwork {
        return db::ensure_artwork_source(
            pool,
            "sidecar",
            None,
            Some(&path.to_string_lossy()),
            None,
            None,
        )
        .await;
    }
    Ok(None)
}

async fn has_sources_for_media(pool: &SqlitePool, media_file_id: i64) -> Result<bool> {
    let row = sqlx::query("SELECT COUNT(*) AS n FROM track_audio_sources WHERE media_file_id = ?")
        .bind(media_file_id)
        .fetch_one(pool)
        .await?;
    Ok(row.try_get::<i64, _>("n")? > 0)
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

fn file_meta(path: &Path) -> Result<(i64, i64)> {
    let meta = std::fs::metadata(path)
        .with_context(|| format!("cue referenced audio file not found: {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok((
            meta.len().try_into().unwrap_or(i64::MAX),
            meta.mtime()
                .saturating_mul(1_000_000_000)
                .saturating_add(meta.mtime_nsec()),
        ))
    }
    #[cfg(not(unix))]
    {
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos().try_into().unwrap_or(i64::MAX))
            .unwrap_or(0);
        Ok((meta.len().try_into().unwrap_or(i64::MAX), mtime))
    }
}

fn normalize_path_key(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| PathBuf::from(path))
        .to_string_lossy()
        .to_string()
}
