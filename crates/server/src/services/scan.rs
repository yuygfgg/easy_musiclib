use crate::application::scan::{
    ArtistNameParser, AudioMetadataReader, AudioTags, CUE_FORMAT_ID, CueRendererSelector, CueSheet,
    CueSheetReader, DiscoveredAudioFile, DiscoveredCueFile, LibraryFileDiscovery, NewTrack,
    NewTrackAudioSource, ScanLibraryRepository,
};
use crate::application::scan_jobs::{self as scan_jobs_app, ScanJobRepository};
use crate::domain::{ScanJobId, ScanJobState};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn spawn_scan<J, L, D, M, C, R, P>(
    scan_jobs: J,
    library: L,
    file_discovery: D,
    metadata_reader: M,
    cue_reader: C,
    renderer_selector: R,
    artist_name_parser: P,
    job_id: ScanJobId,
    roots: Vec<String>,
) where
    J: ScanJobRepository + Clone + Send + Sync + 'static,
    L: ScanLibraryRepository + Clone + Send + Sync + 'static,
    D: LibraryFileDiscovery + Clone + Send + Sync + 'static,
    M: AudioMetadataReader + Clone + Send + Sync + 'static,
    C: CueSheetReader + Clone + Send + Sync + 'static,
    R: CueRendererSelector + Clone + Send + Sync + 'static,
    P: ArtistNameParser + Clone + Send + Sync + 'static,
{
    tokio::spawn(async move {
        if let Err(err) = run_scan(
            &scan_jobs,
            &library,
            &file_discovery,
            &metadata_reader,
            &cue_reader,
            &renderer_selector,
            &artist_name_parser,
            job_id,
            roots,
        )
        .await
        {
            tracing::error!(job_id = job_id.raw(), error = %err, "scan failed");
            let _ = scan_jobs_app::update_scan_job_counts(
                &scan_jobs,
                job_id,
                ScanJobState::Failed,
                None,
                None,
                Some(&err.to_string()),
                true,
            )
            .await;
        }
    });
}

pub async fn run_scan<J, L, D, M, C, R, P>(
    scan_jobs: &J,
    library: &L,
    file_discovery: &D,
    metadata_reader: &M,
    cue_reader: &C,
    renderer_selector: &R,
    artist_name_parser: &P,
    job_id: ScanJobId,
    roots: Vec<String>,
) -> Result<()>
where
    J: ScanJobRepository,
    L: ScanLibraryRepository,
    D: LibraryFileDiscovery,
    M: AudioMetadataReader,
    C: CueSheetReader,
    R: CueRendererSelector,
    P: ArtistNameParser,
{
    scan_jobs_app::update_scan_job_counts(
        scan_jobs,
        job_id,
        ScanJobState::Running,
        None,
        Some(0),
        None,
        false,
    )
    .await?;
    library.ensure_default_split_exceptions().await?;

    let discovered = file_discovery.discover_library_files(&roots).await?;
    scan_jobs_app::update_scan_job_counts(
        scan_jobs,
        job_id,
        ScanJobState::Running,
        Some(discovered.len() as i64),
        Some(0),
        None,
        false,
    )
    .await?;

    let mut cue_audio_paths = HashSet::new();
    for file in &discovered.cues {
        if let Ok(sheet) = cue_reader.parse_cue_file(&file.path).await {
            cue_audio_paths.insert(normalize_path_key(&sheet.audio_path));
        }
    }

    let exceptions = library.split_exceptions().await?;
    let mut scanned = 0i64;

    for file in &discovered.cues {
        if let Err(err) = process_cue(
            library,
            metadata_reader,
            cue_reader,
            renderer_selector,
            artist_name_parser,
            file,
            &exceptions,
        )
        .await
        {
            tracing::error!(path = %file.path.display(), error = %err, "failed to process cue file");
        }
        scanned += 1;
        scan_jobs_app::update_scan_job_counts(
            scan_jobs,
            job_id,
            ScanJobState::Running,
            None,
            Some(scanned),
            None,
            false,
        )
        .await?;
    }

    for file in &discovered.audio {
        if cue_audio_paths.contains(&normalize_path_key(&file.path)) {
            scanned += 1;
            scan_jobs_app::update_scan_job_counts(
                scan_jobs,
                job_id,
                ScanJobState::Running,
                None,
                Some(scanned),
                None,
                false,
            )
            .await?;
            continue;
        }
        if let Err(err) = process_audio_file(
            library,
            metadata_reader,
            renderer_selector,
            file,
            &exceptions,
        )
        .await
        {
            tracing::error!(path = %file.path.display(), error = %err, "failed to process audio file");
        }
        scanned += 1;
        scan_jobs_app::update_scan_job_counts(
            scan_jobs,
            job_id,
            ScanJobState::Running,
            None,
            Some(scanned),
            None,
            false,
        )
        .await?;
    }

    library.discard_unknown_events().await?;
    library.repair_event_dates_and_artwork().await?;
    library.rebuild_relations().await?;
    library.auto_merge().await.ok();
    scan_jobs_app::update_scan_job_counts(
        scan_jobs,
        job_id,
        ScanJobState::Completed,
        None,
        Some(scanned),
        None,
        true,
    )
    .await?;
    Ok(())
}

async fn process_audio_file<L, M, R>(
    library: &L,
    metadata_reader: &M,
    renderer_selector: &R,
    file: &DiscoveredAudioFile,
    exceptions: &[String],
) -> Result<()>
where
    L: ScanLibraryRepository,
    M: AudioMetadataReader,
    R: CueRendererSelector,
{
    let path_string = file.path.to_string_lossy().to_string();
    let (media_file_id, changed) = library
        .upsert_media_file(
            &path_string,
            &file.path_hash,
            file.size,
            file.mtime_ns,
            &file.format,
        )
        .await?;
    if !changed && library.media_file_has_audio_sources(media_file_id).await? {
        return Ok(());
    }
    library.delete_tracks_for_media_file(media_file_id).await?;
    match metadata_reader
        .read_audio_metadata(&file.path, exceptions)
        .await
    {
        Ok(tags) => {
            library
                .set_media_file_audio_metadata(
                    media_file_id,
                    tags.sample_rate,
                    tags.channels,
                    tags.duration_ms,
                )
                .await?;
            write_audio_track(
                library,
                renderer_selector,
                media_file_id,
                &file.format,
                tags,
            )
            .await?;
        }
        Err(err) => {
            library
                .set_media_file_scan_error(media_file_id, &err.to_string())
                .await?;
        }
    }
    Ok(())
}

async fn write_audio_track<L, R>(
    library: &L,
    renderer_selector: &R,
    media_file_id: i64,
    format: &str,
    tags: AudioTags,
) -> Result<i64>
where
    L: ScanLibraryRepository,
    R: CueRendererSelector,
{
    let artwork_id = artwork_for_tags(library, media_file_id, &tags).await?;
    let artist_ids = ensure_artists(library, &tags.artists, artwork_id).await?;
    let album_artist_ids = ensure_artists(library, &tags.album_artists, artwork_id).await?;
    let event_name = tags
        .event
        .as_deref()
        .filter(|name| !library.is_unknown_event_name(name));
    let event_id = library
        .ensure_event(event_name, tags.date.as_deref(), tags.year)
        .await?;
    let album_id = library
        .find_or_create_album(
            &tags.album,
            &album_artist_ids,
            tags.year,
            tags.date.as_deref(),
            event_id,
            artwork_id,
        )
        .await?;
    if let Some(event_id) = event_id {
        library.link_event_album(event_id, album_id).await?;
    }
    let track_id = library
        .insert_track(
            NewTrack {
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
    library
        .insert_track_audio_source(
            track_id,
            NewTrackAudioSource {
                kind: "file",
                media_file_id,
                cue_sheet_id: None,
                codec: format,
                sample_rate: tags.sample_rate,
                start_sample: None,
                end_sample: None,
                start_ms: None,
                end_ms: None,
                renderer: renderer_selector.passthrough_renderer(),
            },
        )
        .await?;
    library.refresh_track_search(track_id).await?;
    library.refresh_album_search(album_id).await?;
    for artist_id in artist_ids.iter().chain(album_artist_ids.iter()) {
        library.refresh_artist_search(*artist_id).await?;
    }
    if let Some(event_id) = event_id {
        library.refresh_event_search(event_id).await?;
    }
    Ok(track_id)
}

async fn process_cue<L, M, C, R, P>(
    library: &L,
    metadata_reader: &M,
    cue_reader: &C,
    renderer_selector: &R,
    artist_name_parser: &P,
    file: &DiscoveredCueFile,
    exceptions: &[String],
) -> Result<()>
where
    L: ScanLibraryRepository,
    M: AudioMetadataReader,
    C: CueSheetReader,
    R: CueRendererSelector,
    P: ArtistNameParser,
{
    let path_string = file.path.to_string_lossy().to_string();
    let (cue_file_id, cue_changed) = library
        .upsert_media_file(
            &path_string,
            &file.path_hash,
            file.size,
            file.mtime_ns,
            CUE_FORMAT_ID,
        )
        .await?;

    let mut sheet = match cue_reader.parse_cue_file(&file.path).await {
        Ok(sheet) => sheet,
        Err(err) => {
            library
                .set_media_file_scan_error(cue_file_id, &err.to_string())
                .await?;
            return Ok(());
        }
    };
    let audio_meta = file_meta(&sheet.audio_path)?;
    let audio_format = format_id_by_extension(&sheet.audio_path).unwrap_or("unknown");
    let audio_path = sheet.audio_path.to_string_lossy().to_string();
    let (audio_file_id, audio_changed) = library
        .upsert_media_file(
            &audio_path,
            &path_hash(&sheet.audio_path),
            audio_meta.0,
            audio_meta.1,
            audio_format,
        )
        .await?;
    let already_has_sheet = library.cue_sheet_exists_for_file(cue_file_id).await?;
    if !cue_changed && !audio_changed && already_has_sheet {
        return Ok(());
    }
    library.delete_cue_sheet_for_file(cue_file_id).await?;

    let audio_tags = metadata_reader
        .read_audio_metadata(&sheet.audio_path, exceptions)
        .await
        .unwrap_or_else(|_| AudioTags {
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
    library
        .set_media_file_audio_metadata(
            audio_file_id,
            audio_tags.sample_rate,
            audio_tags.channels,
            audio_tags.duration_ms,
        )
        .await?;
    apply_audio_timing(&mut sheet, audio_tags.sample_rate, audio_tags.duration_ms);
    let cue_sheet_id = library
        .insert_cue_sheet(
            cue_file_id,
            audio_file_id,
            sheet.album_title.as_deref(),
            sheet.performer.as_deref(),
            sheet.date.as_deref(),
        )
        .await?;

    let artwork_id = artwork_for_tags(library, audio_file_id, &audio_tags).await?;
    let album_name = sheet
        .album_title
        .as_deref()
        .unwrap_or(audio_tags.album.as_str());
    let album_artist_names = sheet
        .performer
        .as_ref()
        .map(|s| vec![s.clone()])
        .unwrap_or_else(|| audio_tags.album_artists.clone());
    let album_artist_names = artist_name_parser.parse_artists(&album_artist_names, exceptions);
    let album_artist_ids = ensure_artists(library, &album_artist_names, artwork_id).await?;
    let date = sheet.date.as_deref().or(audio_tags.date.as_deref());
    let year = cue_year(&sheet).or(audio_tags.year);
    let event_name = audio_tags
        .event
        .as_deref()
        .filter(|name| !library.is_unknown_event_name(name));
    let event_id = library.ensure_event(event_name, date, year).await?;
    let album_id = library
        .find_or_create_album(
            album_name,
            &album_artist_ids,
            year,
            date,
            event_id,
            artwork_id,
        )
        .await?;
    if let Some(event_id) = event_id {
        library.link_event_album(event_id, album_id).await?;
    }

    for cue_track in &sheet.tracks {
        let artist_names = cue_track
            .performer
            .as_ref()
            .map(|s| vec![s.clone()])
            .unwrap_or_else(|| audio_tags.artists.clone());
        let artist_names = artist_name_parser.parse_artists(&artist_names, exceptions);
        let artist_ids = ensure_artists(library, &artist_names, artwork_id).await?;
        let title = cue_track
            .title
            .as_deref()
            .unwrap_or_else(|| audio_tags.title.as_str());
        let duration_ms = cue_track
            .end_ms
            .map(|end| end.saturating_sub(cue_track.start_ms));
        let track_id = library
            .insert_track(
                NewTrack {
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
        let renderer = renderer_selector.cue_renderer_id_for_format_id(audio_format);
        library
            .insert_track_audio_source(
                track_id,
                NewTrackAudioSource {
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
        library.refresh_track_search(track_id).await?;
        for artist_id in artist_ids {
            library.refresh_artist_search(artist_id).await?;
        }
    }
    library.refresh_album_search(album_id).await?;
    for artist_id in album_artist_ids {
        library.refresh_artist_search(artist_id).await?;
    }
    if let Some(event_id) = event_id {
        library.refresh_event_search(event_id).await?;
    }
    Ok(())
}

async fn ensure_artists<L>(
    library: &L,
    names: &[String],
    artwork_id: Option<i64>,
) -> Result<Vec<i64>>
where
    L: ScanLibraryRepository,
{
    let mut out = Vec::new();
    for name in names.iter().filter(|name| !name.trim().is_empty()) {
        let id = library.ensure_artist(name, artwork_id).await?;
        out.push(id);
    }
    if out.is_empty() {
        out.push(library.ensure_artist("Unknown Artist", artwork_id).await?);
    }
    Ok(out)
}

async fn artwork_for_tags<L>(
    library: &L,
    media_file_id: i64,
    tags: &AudioTags,
) -> Result<Option<i64>>
where
    L: ScanLibraryRepository,
{
    if let Some(path) = &tags.sidecar_artwork {
        return library
            .ensure_artwork_source("sidecar", None, Some(&path.to_string_lossy()), None, None)
            .await;
    }
    if let Some(pic) = &tags.embedded_picture {
        return library
            .ensure_artwork_source(
                "embedded",
                Some(media_file_id),
                None,
                Some(pic.index),
                pic.mime.as_deref(),
            )
            .await;
    }
    Ok(None)
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

fn path_hash(path: &Path) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    hex::encode(hasher.finalize())
}

fn format_id_by_extension(path: &Path) -> Option<&'static str> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| match ext.to_ascii_lowercase().as_str() {
            "flac" => Some("flac"),
            "mp3" => Some("mp3"),
            "wav" | "wave" => Some("wav"),
            _ => None,
        })
}

fn apply_audio_timing(sheet: &mut CueSheet, sample_rate: Option<i64>, duration_ms: Option<i64>) {
    let sample_rate = sample_rate.unwrap_or(44_100);
    for idx in 0..sheet.tracks.len() {
        let start_frames = sheet.tracks[idx].start_frames;
        let end_ms = sheet
            .tracks
            .get(idx + 1)
            .map(|next| next.start_ms)
            .or(duration_ms);
        let end_frames = sheet
            .tracks
            .get(idx + 1)
            .map(|next| next.start_frames)
            .or_else(|| duration_ms.map(|ms| ms * 75 / 1000));

        sheet.tracks[idx].start_sample = Some(start_frames * sample_rate / 75);
        sheet.tracks[idx].end_ms = end_ms;
        sheet.tracks[idx].end_sample = end_frames.map(|frames| frames * sample_rate / 75);
    }
}

fn cue_year(sheet: &CueSheet) -> Option<i64> {
    extract_year(sheet.date.as_deref())
}

fn extract_year(input: Option<&str>) -> Option<i64> {
    let input = input?;
    let mut digits = String::new();
    for ch in input.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            if digits.len() == 4 {
                return digits.parse::<i64>().ok();
            }
        } else {
            digits.clear();
        }
    }
    None
}

fn normalize_path_key(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| PathBuf::from(path))
        .to_string_lossy()
        .to_string()
}
