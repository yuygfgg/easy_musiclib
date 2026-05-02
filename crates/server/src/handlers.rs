use crate::db;
use crate::lyrics;
use crate::scanner;
use crate::{ApiResult, AppError, AppState};
use anyhow::Context;
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE,
    RANGE,
};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use easy_musiclib_media::formats::{
    PASSTHROUGH_RENDERER, is_playable_renderer, read_audio_metadata,
    read_embedded_picture_for_path, render_cue_track_by_renderer, render_flac_48k_hls,
    transcode_file_range_for_browser, transcode_file_range_for_browser_to_fd,
};
use easy_musiclib_media::render::{
    FLAC_HLS_INIT_FILE, FLAC_HLS_MEDIA_MIME, FLAC_HLS_PLAYLIST_FILE, FLAC_HLS_PLAYLIST_MIME,
    PlaybackTranscodeFormat, RenderTags,
};
use easy_musiclib_shared::*;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::collections::HashSet;
use std::io::{Cursor, ErrorKind};
use std::os::fd::IntoRawFd;
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::{Path as FsPath, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::time::{Duration, Instant, sleep};
use tokio_util::io::ReaderStream;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    cursor: Option<i64>,
    offset: Option<i64>,
    limit: Option<i64>,
    artist_id: Option<String>,
    album_id: Option<String>,
    event_id: Option<String>,
    liked: Option<bool>,
    q: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    q: String,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RelationQuery {
    artist_id: Option<String>,
    scope: Option<String>,
    depth: Option<i64>,
    limit_nodes: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ArtworkQuery {
    size: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct StreamQuery {
    start_ms: Option<i64>,
    buffered: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct LyricsQuery {
    track_id: Option<String>,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    duration_ms: Option<i64>,
}

pub async fn index(State(state): State<AppState>) -> impl IntoResponse {
    let path = state.static_dir.join("index.html");
    match tokio::fs::read_to_string(path).await {
        Ok(html) => Html(html).into_response(),
        Err(_) => Html(crate::app::FALLBACK_HTML).into_response(),
    }
}

pub async fn list_tracks(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<ListResponse<TrackSummary>>> {
    let artist_id = resolve_opt(&state, "artists", q.artist_id).await?;
    let album_id = resolve_opt(&state, "albums", q.album_id).await?;
    let event_id = resolve_opt(&state, "events", q.event_id).await?;
    Ok(Json(
        db::list_tracks(
            &state.pool,
            q.cursor,
            q.offset,
            q.limit.unwrap_or(50),
            artist_id,
            album_id,
            event_id,
            q.liked,
            q.q,
        )
        .await?,
    ))
}

pub async fn get_track(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<TrackDetail>> {
    let id = db::resolve_id(&state.pool, "tracks", &id).await?;
    Ok(Json(fetch_track_detail_with_duration(&state, id).await?))
}

pub async fn list_albums(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<ListResponse<AlbumSummary>>> {
    let artist_id = resolve_opt(&state, "artists", q.artist_id).await?;
    let event_id = resolve_opt(&state, "events", q.event_id).await?;
    Ok(Json(
        db::list_albums(
            &state.pool,
            q.cursor,
            q.offset,
            q.limit.unwrap_or(50),
            artist_id,
            event_id,
            q.liked,
            q.q,
        )
        .await?,
    ))
}

pub async fn get_album(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<AlbumDetail>> {
    let id = db::resolve_id(&state.pool, "albums", &id).await?;
    Ok(Json(db::fetch_album_detail(&state.pool, id).await?))
}

pub async fn list_artists(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<ListResponse<ArtistSummary>>> {
    Ok(Json(
        db::list_artists(
            &state.pool,
            q.cursor,
            q.offset,
            q.limit.unwrap_or(50),
            q.liked,
            q.q,
        )
        .await?,
    ))
}

pub async fn get_artist(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ArtistDetail>> {
    let id = db::resolve_id(&state.pool, "artists", &id).await?;
    Ok(Json(db::fetch_artist_detail(&state.pool, id).await?))
}

pub async fn list_events(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<ListResponse<EventSummary>>> {
    Ok(Json(
        db::list_events(
            &state.pool,
            q.cursor,
            q.offset,
            q.limit.unwrap_or(50),
            q.liked,
            q.q,
        )
        .await?,
    ))
}

pub async fn get_event(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<EventDetail>> {
    let id = db::resolve_id(&state.pool, "events", &id).await?;
    Ok(Json(db::fetch_event_detail(&state.pool, id).await?))
}

pub async fn search(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> ApiResult<Json<SearchResponse>> {
    Ok(Json(
        db::search(&state.pool, &q.q, q.limit.unwrap_or(50)).await?,
    ))
}

pub async fn relations(
    State(state): State<AppState>,
    Query(q): Query<RelationQuery>,
) -> ApiResult<Json<RelationGraph>> {
    let artist_id = if q.scope.as_deref() == Some("all") {
        None
    } else {
        resolve_opt(&state, "artists", q.artist_id).await?
    };
    Ok(Json(
        db::relation_graph(
            &state.pool,
            artist_id,
            q.depth.unwrap_or(2),
            q.limit_nodes.unwrap_or(500),
        )
        .await?,
    ))
}

pub async fn patch_track(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(patch): Json<LikePatch>,
) -> ApiResult<Json<TrackDetail>> {
    let id = db::resolve_id(&state.pool, "tracks", &id).await?;
    db::set_liked(&state.pool, "tracks", id, patch.liked).await?;
    Ok(Json(fetch_track_detail_with_duration(&state, id).await?))
}

pub async fn patch_album(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(patch): Json<LikePatch>,
) -> ApiResult<Json<AlbumDetail>> {
    let id = db::resolve_id(&state.pool, "albums", &id).await?;
    db::set_liked(&state.pool, "albums", id, patch.liked).await?;
    Ok(Json(db::fetch_album_detail(&state.pool, id).await?))
}

pub async fn patch_artist(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(patch): Json<LikePatch>,
) -> ApiResult<Json<ArtistDetail>> {
    let id = db::resolve_id(&state.pool, "artists", &id).await?;
    db::set_liked(&state.pool, "artists", id, patch.liked).await?;
    Ok(Json(db::fetch_artist_detail(&state.pool, id).await?))
}

pub async fn patch_event(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(patch): Json<LikePatch>,
) -> ApiResult<Json<EventDetail>> {
    let id = db::resolve_id(&state.pool, "events", &id).await?;
    db::set_liked(&state.pool, "events", id, patch.liked).await?;
    Ok(Json(db::fetch_event_detail(&state.pool, id).await?))
}

async fn fetch_track_detail_with_duration(state: &AppState, id: i64) -> ApiResult<TrackDetail> {
    ensure_track_duration_ms(state, id).await?;
    Ok(db::fetch_track_detail(&state.pool, id).await?)
}

async fn ensure_track_duration_ms(state: &AppState, id: i64) -> ApiResult<()> {
    let Some(source) = track_duration_source(state, id).await? else {
        return Ok(());
    };
    if source.track_duration_ms.is_some() {
        return Ok(());
    }

    match infer_track_duration_ms(&source).await {
        Ok(Some(duration_ms)) => {
            persist_track_duration_ms(state, &source, duration_ms).await?;
        }
        Ok(None) => {}
        Err(err) => {
            tracing::warn!(
                track_id = id,
                path = %source.path.as_deref().unwrap_or(""),
                error = %err,
                "failed to infer track duration"
            );
        }
    }
    Ok(())
}

async fn track_duration_source(
    state: &AppState,
    id: i64,
) -> ApiResult<Option<TrackDurationSource>> {
    let row = sqlx::query(
        "SELECT
            t.duration_ms AS track_duration_ms,
            tas.kind, tas.media_file_id, tas.sample_rate, tas.start_sample, tas.end_sample,
            tas.start_ms, tas.end_ms,
            mf.path, mf.duration_ms AS media_duration_ms
         FROM tracks t
         LEFT JOIN track_audio_sources tas ON tas.track_id = t.id
         LEFT JOIN media_files mf ON mf.id = tas.media_file_id
         WHERE t.id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(TrackDurationSource {
        track_id: id,
        track_duration_ms: row.try_get("track_duration_ms")?,
        kind: row.try_get("kind")?,
        media_file_id: row.try_get("media_file_id")?,
        path: row.try_get("path")?,
        media_duration_ms: row.try_get("media_duration_ms")?,
        sample_rate: row.try_get("sample_rate")?,
        start_sample: row.try_get("start_sample")?,
        end_sample: row.try_get("end_sample")?,
        start_ms: row.try_get("start_ms")?,
        end_ms: row.try_get("end_ms")?,
    }))
}

async fn infer_track_duration_ms(source: &TrackDurationSource) -> anyhow::Result<Option<i64>> {
    if let (Some(start_ms), Some(end_ms)) = (source.start_ms, source.end_ms) {
        return Ok(positive_duration(end_ms.saturating_sub(start_ms)));
    }
    if let (Some(sample_rate), Some(start_sample), Some(end_sample)) =
        (source.sample_rate, source.start_sample, source.end_sample)
    {
        if sample_rate > 0 {
            return Ok(positive_duration(
                end_sample.saturating_sub(start_sample).saturating_mul(1000) / sample_rate,
            ));
        }
    }

    let file_duration_ms = match source.media_duration_ms {
        Some(duration_ms) => Some(duration_ms),
        None => read_source_duration_ms(source).await?,
    };
    let Some(file_duration_ms) = file_duration_ms else {
        return Ok(None);
    };
    if source.kind.as_deref() == Some("cue") {
        let start_ms = source
            .start_ms
            .or_else(|| cue_start_ms_from_samples(source))
            .unwrap_or(0);
        return Ok(positive_duration(file_duration_ms.saturating_sub(start_ms)));
    }
    Ok(positive_duration(file_duration_ms))
}

async fn read_source_duration_ms(source: &TrackDurationSource) -> anyhow::Result<Option<i64>> {
    let Some(path) = source.path.clone() else {
        return Ok(None);
    };
    let tags = tokio::task::spawn_blocking(move || {
        let path = PathBuf::from(path);
        read_audio_metadata(&path, &[])
    })
    .await
    .map_err(|e| anyhow::anyhow!(e.to_string()))??;
    Ok(tags.duration_ms)
}

fn cue_start_ms_from_samples(source: &TrackDurationSource) -> Option<i64> {
    let sample_rate = source.sample_rate?;
    let start_sample = source.start_sample?;
    (sample_rate > 0).then_some(start_sample.saturating_mul(1000) / sample_rate)
}

fn positive_duration(duration_ms: i64) -> Option<i64> {
    (duration_ms > 0).then_some(duration_ms)
}

async fn persist_track_duration_ms(
    state: &AppState,
    source: &TrackDurationSource,
    duration_ms: i64,
) -> ApiResult<()> {
    sqlx::query(
        "UPDATE tracks
         SET duration_ms = ?
         WHERE id = ? AND duration_ms IS NULL",
    )
    .bind(duration_ms)
    .bind(source.track_id)
    .execute(&state.pool)
    .await?;

    if source.kind.as_deref() == Some("cue") {
        if source.end_ms.is_none() {
            if let Some(start_ms) = source
                .start_ms
                .or_else(|| cue_start_ms_from_samples(source))
            {
                sqlx::query(
                    "UPDATE track_audio_sources
                     SET start_ms = COALESCE(start_ms, ?), end_ms = ?
                     WHERE track_id = ? AND end_ms IS NULL",
                )
                .bind(start_ms)
                .bind(start_ms.saturating_add(duration_ms))
                .bind(source.track_id)
                .execute(&state.pool)
                .await?;
            }
        }
    } else if source.media_duration_ms.is_none() {
        if let Some(media_file_id) = source.media_file_id {
            sqlx::query(
                "UPDATE media_files
                 SET duration_ms = ?
                 WHERE id = ? AND duration_ms IS NULL",
            )
            .bind(duration_ms)
            .bind(media_file_id)
            .execute(&state.pool)
            .await?;
        }
    }
    Ok(())
}

struct TrackDurationSource {
    track_id: i64,
    track_duration_ms: Option<i64>,
    kind: Option<String>,
    media_file_id: Option<i64>,
    path: Option<String>,
    media_duration_ms: Option<i64>,
    sample_rate: Option<i64>,
    start_sample: Option<i64>,
    end_sample: Option<i64>,
    start_ms: Option<i64>,
    end_ms: Option<i64>,
}

pub async fn create_artist(
    State(state): State<AppState>,
    Json(req): Json<CreateArtistRequest>,
) -> ApiResult<Json<ArtistSummary>> {
    Ok(Json(db::create_artist(&state.pool, &req.name).await?))
}

pub async fn add_alias(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<AddAliasRequest>,
) -> ApiResult<Json<MessageResponse>> {
    let id = db::resolve_id(&state.pool, "artists", &id).await?;
    db::add_artist_alias(&state.pool, id, &req.alias).await?;
    Ok(Json(MessageResponse {
        message: "alias added".to_string(),
    }))
}

pub async fn merge_artists(
    State(state): State<AppState>,
    Json(req): Json<MergeArtistsRequest>,
) -> ApiResult<Json<MessageResponse>> {
    let target = if req.by_name {
        db::ensure_artist(&state.pool, &req.target, None).await?
    } else {
        db::resolve_id(&state.pool, "artists", &req.target).await?
    };
    let source = if req.by_name {
        match db::resolve_id(&state.pool, "artists", &req.source).await {
            Ok(id) => id,
            Err(_) => db::ensure_artist(&state.pool, &req.source, None).await?,
        }
    } else {
        db::resolve_id(&state.pool, "artists", &req.source).await?
    };
    db::merge_artists(&state.pool, target, source, "manual").await?;
    Ok(Json(MessageResponse {
        message: "artists merged".to_string(),
    }))
}

pub async fn auto_merge(State(state): State<AppState>) -> ApiResult<Json<MessageResponse>> {
    let count = db::auto_merge(&state.pool).await?;
    Ok(Json(MessageResponse {
        message: format!("auto merge completed: {count}"),
    }))
}

pub async fn alias_csv_import(
    State(state): State<AppState>,
    Json(req): Json<AliasCsvImportRequest>,
) -> ApiResult<Json<MessageResponse>> {
    let count = db::import_alias_csv(&state.pool, &req.csv).await?;
    Ok(Json(MessageResponse {
        message: format!("alias csv imported: {count} merges"),
    }))
}

pub async fn create_scan_job(
    State(state): State<AppState>,
    Json(req): Json<ScanJobRequest>,
) -> ApiResult<Json<ScanJobStatus>> {
    if req.roots.is_empty() {
        return Err(AppError::bad_request("roots is empty"));
    }
    let job = db::insert_or_update_scan_job(&state.pool, &req.roots).await?;
    scanner::spawn_scan(state.pool.clone(), job.id, req.roots);
    Ok(Json(job))
}

pub async fn get_scan_job(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<ScanJobStatus>> {
    Ok(Json(db::scan_job(&state.pool, id).await?))
}

pub async fn cancel_scan_job(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<MessageResponse>> {
    // TODO: stop the scan job
    db::update_scan_job_counts(&state.pool, id, "cancel_requested", None, None, None, false)
        .await?;
    Ok(Json(MessageResponse {
        message: "cancel requested".to_string(),
    }))
}

pub async fn vacuum(State(state): State<AppState>) -> ApiResult<Json<MessageResponse>> {
    sqlx::query("VACUUM").execute(&state.pool).await?;
    Ok(Json(MessageResponse {
        message: "database vacuum completed".to_string(),
    }))
}

pub async fn lyrics_search(
    State(state): State<AppState>,
    Query(q): Query<LyricsQuery>,
) -> ApiResult<Json<Vec<LyricsCandidate>>> {
    let (track_id, title, artist, album, duration_ms) = if let Some(track_id) = q.track_id {
        let id = db::resolve_id(&state.pool, "tracks", &track_id).await?;
        let detail = fetch_track_detail_with_duration(&state, id).await?;
        (
            Some(id),
            detail.summary.title,
            detail
                .summary
                .artists
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            detail.summary.album.map(|a| a.name),
            detail.summary.duration_ms,
        )
    } else {
        (
            None,
            q.title
                .ok_or_else(|| AppError::bad_request("title is required"))?,
            q.artist.unwrap_or_default(),
            q.album,
            q.duration_ms,
        )
    };

    let cached = cached_lyrics(&state, track_id, &title, &artist).await?;
    if !cached.is_empty() {
        return Ok(Json(cached));
    }
    let results = lyrics::search_netease(&title, &artist, album.as_deref(), duration_ms)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    for item in &results {
        db::cache_lyrics(&state.pool, track_id, item).await.ok();
    }
    Ok(Json(results))
}

pub async fn get_settings(State(state): State<AppState>) -> ApiResult<Json<AppSettings>> {
    Ok(Json(db::app_settings(&state.pool).await?))
}

pub async fn patch_settings(
    State(state): State<AppState>,
    Json(req): Json<UpdateAppSettingsRequest>,
) -> ApiResult<Json<AppSettings>> {
    Ok(Json(db::update_app_settings(&state.pool, req).await?))
}

pub async fn clear_hls_cache() -> ApiResult<Json<HlsCacheClearResponse>> {
    let root = hls_cache_root();
    let summary = tokio::task::spawn_blocking(move || {
        let active = hls_generators()
            .lock()
            .map_err(|_| anyhow::anyhow!("HLS generator lock poisoned"))?;
        clear_hls_cache_root(&root, &active)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))??;
    Ok(Json(summary))
}

async fn cached_lyrics(
    state: &AppState,
    track_id: Option<i64>,
    title: &str,
    artist: &str,
) -> ApiResult<Vec<LyricsCandidate>> {
    let rows = if let Some(track_id) = track_id {
        sqlx::query(
            "SELECT title, artist, album, duration_ms, provider, lyrics, score
             FROM lyric_cache WHERE track_id = ? ORDER BY score DESC LIMIT 9",
        )
        .bind(track_id)
        .fetch_all(&state.pool)
        .await?
    } else {
        sqlx::query(
            "SELECT title, artist, album, duration_ms, provider, lyrics, score
             FROM lyric_cache WHERE title = ? AND artist = ? ORDER BY score DESC LIMIT 9",
        )
        .bind(title)
        .bind(artist)
        .fetch_all(&state.pool)
        .await?
    };
    rows.into_iter()
        .map(|row| {
            Ok(LyricsCandidate {
                title: row.try_get("title")?,
                artist: row.try_get("artist")?,
                album: row.try_get("album")?,
                duration_ms: row.try_get("duration_ms")?,
                provider: row.try_get("provider")?,
                lyrics: row.try_get("lyrics")?,
                score: row.try_get("score")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(Into::into)
}

pub async fn artwork(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(q): Query<ArtworkQuery>,
) -> ApiResult<Response> {
    let size = q.size.unwrap_or(256).clamp(32, 2000);
    let variant = format!("size={size}");
    if let Some((bytes, mime)) = db::get_artwork_blob(&state.pool, id, &variant).await? {
        return Ok(binary_response(StatusCode::OK, bytes, &mime, None, true));
    }
    let source = db::source_for_artwork(&state.pool, id).await?;
    let (raw, _mime) = match source.kind.as_str() {
        "sidecar" => {
            let path = source
                .sidecar_path
                .ok_or_else(|| AppError::not_found("artwork source has no sidecar path"))?;
            let bytes = tokio::fs::read(&path).await?;
            let mime = mime_guess::from_path(&path)
                .first_raw()
                .unwrap_or("application/octet-stream")
                .to_string();
            (bytes, mime)
        }
        "embedded" => {
            let path = source
                .media_path
                .ok_or_else(|| AppError::not_found("artwork source has no media path"))?;
            tokio::task::spawn_blocking(move || {
                read_embedded_picture_for_path(
                    std::path::Path::new(&path),
                    source.embedded_picture_index.unwrap_or(0),
                )
            })
            .await
            .map_err(|e| AppError::internal(e.to_string()))?
            .map(|(bytes, mime)| (bytes, mime.unwrap_or_else(|| "image/jpeg".to_string())))?
        }
        _ => return Err(AppError::not_found("unsupported artwork source")),
    };
    let resized = tokio::task::spawn_blocking(move || resize_image(raw, size))
        .await
        .map_err(|e| AppError::internal(e.to_string()))??;
    db::put_artwork_blob(
        &state.pool,
        id,
        &variant,
        "image/jpeg",
        Some(size as i64),
        None,
        resized.clone(),
    )
    .await
    .ok();
    Ok(binary_response(
        StatusCode::OK,
        resized,
        "image/jpeg",
        None,
        true,
    ))
}

pub async fn stream_track(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<StreamQuery>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    stream_track_response(
        state,
        id,
        q.start_ms.unwrap_or(0),
        q.buffered.unwrap_or(false),
        headers,
    )
    .await
}

pub async fn stream_track_hls_file(
    State(state): State<AppState>,
    Path((id, file)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let id = db::resolve_id(&state.pool, "tracks", &id).await?;
    let source = db::track_render_source(&state.pool, id).await?;
    if !is_playable_renderer(Some(&source.renderer)) {
        return Err(AppError::bad_request("track is not playable"));
    }
    let cache_dir = hls_cache_dir(id, &source).await?;
    let path = hls_file_path(&cache_dir, &file)?;
    ensure_hls_generation(&source, &cache_dir).await?;
    wait_for_hls_file(&path, hls_file_timeout(&file)).await?;
    let mime = hls_file_mime(&file)?;
    if file == FLAC_HLS_PLAYLIST_FILE {
        return hls_playlist_response(&path).await;
    }
    let mut response = ranged_file_response(&path, mime, None, &headers, false).await?;
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    Ok(response)
}

pub async fn download_track(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    render_track_response(state, id, headers, true).await
}

async fn stream_track_response(
    state: AppState,
    id: String,
    requested_start_ms: i64,
    buffered: bool,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let id = db::resolve_id(&state.pool, "tracks", &id).await?;
    let source = db::track_render_source(&state.pool, id).await?;
    if !is_playable_renderer(Some(&source.renderer)) {
        return Err(AppError::bad_request("track is not playable"));
    }
    ensure_track_duration_ms(&state, id).await?;
    let source = db::track_render_source(&state.pool, id).await?;
    let playback_format =
        media_playback_format(db::app_settings(&state.pool).await?.browser_playback_format);
    stream_browser_audio(
        source,
        playback_format,
        requested_start_ms,
        buffered,
        headers,
    )
    .await
}

async fn hls_cache_dir(track_id: i64, source: &db::RenderSourceRow) -> ApiResult<PathBuf> {
    let metadata = tokio::fs::metadata(&source.path).await?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let mut hasher = Sha256::new();
    hasher.update(b"flac-48k-fmp4-hls-v1");
    hasher.update(track_id.to_le_bytes());
    hasher.update(source.path.as_bytes());
    hasher.update(source.renderer.as_bytes());
    hasher.update(source.codec.as_bytes());
    hasher.update(source.start_ms.unwrap_or(0).to_le_bytes());
    hasher.update(source.end_ms.unwrap_or(-1).to_le_bytes());
    hasher.update(metadata.len().to_le_bytes());
    hasher.update(modified.to_le_bytes());
    Ok(hls_cache_root().join(hex::encode(hasher.finalize())))
}

fn hls_cache_root() -> PathBuf {
    std::env::temp_dir().join("easy_musiclib_hls")
}

fn hls_file_path(cache_dir: &FsPath, file: &str) -> ApiResult<PathBuf> {
    if file.contains('/') || file.contains('\\') || file.contains("..") {
        return Err(AppError::bad_request("invalid HLS file name"));
    }
    if file == FLAC_HLS_PLAYLIST_FILE || file == FLAC_HLS_INIT_FILE || is_hls_segment_file(file) {
        return Ok(cache_dir.join(file));
    }
    Err(AppError::not_found("HLS file not found"))
}

fn is_hls_segment_file(file: &str) -> bool {
    let Some(index) = file
        .strip_prefix("segment_")
        .and_then(|rest| rest.strip_suffix(".m4s"))
    else {
        return false;
    };
    index.len() == 5 && index.bytes().all(|byte| byte.is_ascii_digit())
}

fn hls_file_mime(file: &str) -> ApiResult<&'static str> {
    if file == FLAC_HLS_PLAYLIST_FILE {
        Ok(FLAC_HLS_PLAYLIST_MIME)
    } else if file == FLAC_HLS_INIT_FILE || is_hls_segment_file(file) {
        Ok(FLAC_HLS_MEDIA_MIME)
    } else {
        Err(AppError::not_found("HLS file not found"))
    }
}

fn hls_file_timeout(file: &str) -> Duration {
    if file == FLAC_HLS_PLAYLIST_FILE || file == FLAC_HLS_INIT_FILE {
        Duration::from_secs(15)
    } else {
        Duration::from_secs(30)
    }
}

async fn hls_playlist_response(path: &FsPath) -> ApiResult<Response> {
    let playlist = tokio::fs::read_to_string(path).await?;
    let playlist = hls_playlist_for_playback(&playlist);
    let len = playlist.len();
    let mut response = (StatusCode::OK, Body::from(playlist)).into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static(FLAC_HLS_PLAYLIST_MIME),
    );
    response.headers_mut().insert(
        CONTENT_LENGTH,
        HeaderValue::from_str(&len.to_string()).unwrap(),
    );
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    Ok(response)
}

fn hls_playlist_for_playback(playlist: &str) -> String {
    if playlist.contains("#EXT-X-START:") {
        return playlist.to_owned();
    }

    let insert_after = playlist
        .lines()
        .position(|line| line.starts_with("#EXT-X-VERSION"))
        .or_else(|| playlist.lines().position(|line| line == "#EXTM3U"));
    let Some(insert_after) = insert_after else {
        return playlist.to_owned();
    };

    let mut out = String::with_capacity(playlist.len() + 48);
    for (index, line) in playlist.lines().enumerate() {
        out.push_str(line);
        out.push('\n');
        if index == insert_after {
            out.push_str("#EXT-X-START:TIME-OFFSET=0.000,PRECISE=YES\n");
        }
    }
    out
}

async fn ensure_hls_generation(source: &db::RenderSourceRow, cache_dir: &FsPath) -> ApiResult<()> {
    if tokio::fs::metadata(hls_complete_path(cache_dir))
        .await
        .is_ok()
    {
        return Ok(());
    }

    let cache_dir = cache_dir.to_path_buf();
    let should_start = {
        let mut active = hls_generators()
            .lock()
            .map_err(|_| AppError::internal("HLS generator lock poisoned"))?;
        active.insert(cache_dir.clone())
    };
    if !should_start {
        return Ok(());
    }

    let input_path = PathBuf::from(source.path.clone());
    let start_ms = source.start_ms.unwrap_or(0).max(0);
    let end_ms = source.end_ms;
    tokio::task::spawn_blocking(move || {
        let result = (|| -> anyhow::Result<()> {
            if cache_dir.exists() {
                std::fs::remove_dir_all(&cache_dir)
                    .with_context(|| format!("removing stale HLS cache {}", cache_dir.display()))?;
            }
            std::fs::create_dir_all(&cache_dir)
                .with_context(|| format!("creating HLS cache {}", cache_dir.display()))?;
            render_flac_48k_hls(&input_path, &cache_dir, start_ms, end_ms)?;
            std::fs::write(hls_complete_path(&cache_dir), b"ok")
                .with_context(|| format!("writing HLS complete marker {}", cache_dir.display()))?;
            Ok(())
        })();
        if let Err(err) = result {
            tracing::error!(
                path = %input_path.display(),
                cache_dir = %cache_dir.display(),
                error = %err,
                "failed to generate FLAC HLS"
            );
        }
        if let Ok(mut active) = hls_generators().lock() {
            active.remove(&cache_dir);
        }
    });

    Ok(())
}

async fn wait_for_hls_file(path: &FsPath, timeout: Duration) -> ApiResult<()> {
    let deadline = Instant::now() + timeout;
    loop {
        if tokio::fs::metadata(path).await.is_ok() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(AppError::not_found("HLS file is not ready"));
        }
        sleep(Duration::from_millis(50)).await;
    }
}

fn hls_complete_path(cache_dir: &FsPath) -> PathBuf {
    cache_dir.join(".complete")
}

fn hls_generators() -> &'static Mutex<HashSet<PathBuf>> {
    static ACTIVE: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    ACTIVE.get_or_init(|| Mutex::new(HashSet::new()))
}

fn clear_hls_cache_root(
    root: &FsPath,
    active_dirs: &HashSet<PathBuf>,
) -> anyhow::Result<HlsCacheClearResponse> {
    let mut summary = HlsCacheClearResponse {
        cache_dir: root.to_string_lossy().into_owned(),
        removed_files: 0,
        removed_dirs: 0,
        removed_bytes: 0,
        skipped_active_generators: 0,
    };

    match std::fs::symlink_metadata(root) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => {
            remove_hls_cache_path(root, &mut summary)?;
            return Ok(summary);
        }
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(summary),
        Err(err) => {
            return Err(err).with_context(|| format!("reading HLS cache root {}", root.display()));
        }
    }

    if active_dirs.is_empty() {
        remove_hls_cache_path(root, &mut summary)?;
        return Ok(summary);
    }

    for entry in std::fs::read_dir(root)
        .with_context(|| format!("reading HLS cache root {}", root.display()))?
    {
        let path = entry
            .with_context(|| format!("reading HLS cache root {}", root.display()))?
            .path();
        if active_dirs.contains(&path) {
            summary.skipped_active_generators += 1;
            continue;
        }
        remove_hls_cache_path(&path, &mut summary)?;
    }

    match std::fs::remove_dir(root) {
        Ok(()) => summary.removed_dirs += 1,
        Err(err)
            if matches!(
                err.kind(),
                ErrorKind::NotFound | ErrorKind::DirectoryNotEmpty
            ) => {}
        Err(err) => {
            return Err(err).with_context(|| format!("removing HLS cache root {}", root.display()));
        }
    }

    Ok(summary)
}

fn remove_hls_cache_path(path: &FsPath, summary: &mut HlsCacheClearResponse) -> anyhow::Result<()> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(err).with_context(|| format!("reading HLS cache path {}", path.display()));
        }
    };

    if metadata.is_dir() {
        for entry in
            std::fs::read_dir(path).with_context(|| format!("reading {}", path.display()))?
        {
            let path = entry
                .with_context(|| format!("reading {}", path.display()))?
                .path();
            remove_hls_cache_path(&path, summary)?;
        }
        match std::fs::remove_dir(path) {
            Ok(()) => summary.removed_dirs += 1,
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("removing HLS cache directory {}", path.display()));
            }
        }
    } else {
        let bytes = metadata.len();
        match std::fs::remove_file(path) {
            Ok(()) => {
                summary.removed_files += 1;
                summary.removed_bytes = summary.removed_bytes.saturating_add(bytes);
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("removing HLS cache file {}", path.display()));
            }
        }
    }

    Ok(())
}

async fn render_track_response(
    state: AppState,
    id: String,
    headers: HeaderMap,
    download: bool,
) -> ApiResult<Response> {
    let id = db::resolve_id(&state.pool, "tracks", &id).await?;
    let source = db::track_render_source(&state.pool, id).await?;
    if source.renderer == PASSTHROUGH_RENDERER {
        return passthrough_file(&source.path, &source.title, headers, download).await;
    }
    if !is_playable_renderer(Some(&source.renderer)) {
        return Err(AppError::bad_request("track is not playable"));
    }

    let tags = RenderTags {
        title: source.title.clone(),
        artist: source.artist.clone(),
        album: source.album.clone(),
        track_no: source.track_no,
        date: source.date.clone(),
    };
    let renderer = source.renderer.clone();
    let path = PathBuf::from(source.path.clone());
    let start_sample = source.start_sample.unwrap_or(0);
    let end_sample = source.end_sample;
    let rendered = tokio::task::spawn_blocking(move || {
        render_cue_track_by_renderer(&renderer, &path, start_sample, end_sample, &tags)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))??;
    Ok(audio_bytes_response(
        rendered.bytes,
        rendered.mime,
        rendered.extension,
        &source.title,
        download,
        &headers,
    ))
}

fn media_playback_format(format: BrowserPlaybackFormat) -> PlaybackTranscodeFormat {
    match format {
        BrowserPlaybackFormat::Opus256k => PlaybackTranscodeFormat::Opus256k,
        BrowserPlaybackFormat::Flac48k => PlaybackTranscodeFormat::Flac48k,
    }
}

async fn stream_browser_audio(
    source: db::RenderSourceRow,
    playback_format: PlaybackTranscodeFormat,
    requested_start_ms: i64,
    buffered: bool,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let (absolute_start_ms, end_ms) = browser_stream_time_range(&source, requested_start_ms);
    // Browsers often send `Range: bytes=0-` for media startup. This endpoint seeks
    // by `start_ms`, so serving byte ranges would require a full transcode first.
    if buffered {
        return buffered_browser_audio(source, playback_format, absolute_start_ms, end_ms, headers)
            .await;
    }

    streaming_browser_audio(source, playback_format, absolute_start_ms, end_ms).await
}

fn browser_stream_time_range(
    source: &db::RenderSourceRow,
    requested_start_ms: i64,
) -> (i64, Option<i64>) {
    let track_start_ms = source.start_ms.unwrap_or(0).max(0);
    let relative_start_ms = requested_start_ms.max(0);
    let mut absolute_start_ms = track_start_ms.saturating_add(relative_start_ms);
    if let Some(end_ms) = source.end_ms {
        absolute_start_ms = absolute_start_ms.min(end_ms.saturating_sub(1).max(track_start_ms));
    }
    (absolute_start_ms, source.end_ms)
}

async fn buffered_browser_audio(
    source: db::RenderSourceRow,
    playback_format: PlaybackTranscodeFormat,
    absolute_start_ms: i64,
    end_ms: Option<i64>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let input_path = PathBuf::from(source.path.clone());
    let title = source.title.clone();
    let rendered = tokio::task::spawn_blocking(move || {
        transcode_file_range_for_browser(&input_path, playback_format, absolute_start_ms, end_ms)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))??;

    Ok(audio_bytes_response(
        rendered.bytes,
        rendered.mime,
        rendered.extension,
        &title,
        false,
        &headers,
    ))
}

async fn streaming_browser_audio(
    source: db::RenderSourceRow,
    playback_format: PlaybackTranscodeFormat,
    absolute_start_ms: i64,
    end_ms: Option<i64>,
) -> ApiResult<Response> {
    let (reader, writer) = StdUnixStream::pair()?;
    reader.set_nonblocking(true)?;
    let reader = tokio::net::UnixStream::from_std(reader)?;
    let output_fd = writer.into_raw_fd();
    let input_path = PathBuf::from(source.path.clone());
    let title = source.title.clone();
    tokio::task::spawn_blocking(move || {
        if let Err(err) = transcode_file_range_for_browser_to_fd(
            &input_path,
            playback_format,
            absolute_start_ms,
            end_ms,
            output_fd,
        ) {
            tracing::debug!(
                path = %input_path.display(),
                start_ms = absolute_start_ms,
                error = %err,
                "browser transcode stream ended with error"
            );
        }
    });

    let stream = ReaderStream::with_capacity(reader, 64 * 1024);
    let mut response = (StatusCode::OK, Body::from_stream(stream)).into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static(playback_format.mime()),
    );
    response
        .headers_mut()
        .insert(ACCEPT_RANGES, HeaderValue::from_static("none"));
    response.headers_mut().insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(
            "inline; filename*=UTF-8''{}.{}",
            percent_encoding::utf8_percent_encode(&title, percent_encoding::NON_ALPHANUMERIC),
            playback_format.extension(),
        ))
        .unwrap(),
    );
    Ok(response)
}

async fn passthrough_file(
    path: &str,
    title: &str,
    headers: HeaderMap,
    download: bool,
) -> ApiResult<Response> {
    let path = PathBuf::from(path);
    let mime = mime_guess::from_path(&path)
        .first_raw()
        .unwrap_or("application/octet-stream")
        .to_string();
    ranged_file_response(&path, &mime, Some((title, "")), &headers, download).await
}

async fn ranged_file_response(
    path: &PathBuf,
    mime: &str,
    download_name: Option<(&str, &str)>,
    headers: &HeaderMap,
    download: bool,
) -> ApiResult<Response> {
    let mut file = tokio::fs::File::open(path).await?;
    let len = file.metadata().await?.len();
    let (status, start, end) = match requested_range(headers, len) {
        RequestedRange::None => (StatusCode::OK, 0, len.saturating_sub(1)),
        RequestedRange::Valid(start, end) => (StatusCode::PARTIAL_CONTENT, start, end),
        RequestedRange::Invalid => return Ok(range_not_satisfiable_response(len, Some(mime))),
    };
    let read_len = if len == 0 {
        0
    } else {
        end.saturating_sub(start).saturating_add(1)
    };
    file.seek(std::io::SeekFrom::Start(start)).await?;
    let stream = ReaderStream::new(file.take(read_len));
    let mut response = (status, Body::from_stream(stream)).into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_str(mime).unwrap());
    response.headers_mut().insert(
        CONTENT_LENGTH,
        HeaderValue::from_str(&read_len.to_string()).unwrap(),
    );
    response
        .headers_mut()
        .insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    if status == StatusCode::PARTIAL_CONTENT {
        response.headers_mut().insert(
            CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {start}-{end}/{len}")).unwrap(),
        );
    }
    if download {
        let Some((title, extension)) = download_name else {
            return Ok(response);
        };
        let suffix = if extension.is_empty() {
            String::new()
        } else {
            format!(".{extension}")
        };
        response.headers_mut().insert(
            CONTENT_DISPOSITION,
            HeaderValue::from_str(&format!(
                "attachment; filename*=UTF-8''{}{}",
                percent_encoding::utf8_percent_encode(title, percent_encoding::NON_ALPHANUMERIC),
                suffix
            ))
            .unwrap(),
        );
    }
    Ok(response)
}

fn audio_bytes_response(
    bytes: Vec<u8>,
    mime: &str,
    extension: &str,
    title: &str,
    download: bool,
    headers: &HeaderMap,
) -> Response {
    let len = bytes.len() as u64;
    let (status, start, end) = if download {
        (StatusCode::OK, 0, len.saturating_sub(1))
    } else {
        match requested_range(headers, len) {
            RequestedRange::None => (StatusCode::OK, 0, len.saturating_sub(1)),
            RequestedRange::Valid(start, end) => (StatusCode::PARTIAL_CONTENT, start, end),
            RequestedRange::Invalid => return range_not_satisfiable_response(len, Some(mime)),
        }
    };
    let body = if len == 0 {
        bytes
    } else {
        bytes[start as usize..=end as usize].to_vec()
    };
    let read_len = body.len();
    let mut response = (status, Body::from(body)).into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_str(mime).unwrap());
    response.headers_mut().insert(
        CONTENT_LENGTH,
        HeaderValue::from_str(&read_len.to_string()).unwrap(),
    );
    response
        .headers_mut()
        .insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    if status == StatusCode::PARTIAL_CONTENT {
        response.headers_mut().insert(
            CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {start}-{end}/{len}")).unwrap(),
        );
    }
    if download {
        response.headers_mut().insert(
            CONTENT_DISPOSITION,
            HeaderValue::from_str(&format!(
                "attachment; filename*=UTF-8''{}.{}",
                percent_encoding::utf8_percent_encode(title, percent_encoding::NON_ALPHANUMERIC),
                extension,
            ))
            .unwrap(),
        );
    }
    response
}

fn range_not_satisfiable_response(len: u64, mime: Option<&str>) -> Response {
    let mut response = (
        StatusCode::RANGE_NOT_SATISFIABLE,
        Body::from(Vec::<u8>::new()),
    )
        .into_response();
    response
        .headers_mut()
        .insert(CONTENT_LENGTH, HeaderValue::from_static("0"));
    response
        .headers_mut()
        .insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    response.headers_mut().insert(
        CONTENT_RANGE,
        HeaderValue::from_str(&format!("bytes */{len}")).unwrap(),
    );
    if let Some(mime) = mime {
        response
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_str(mime).unwrap());
    }
    response
}

fn binary_response(
    status: StatusCode,
    bytes: Vec<u8>,
    mime: &str,
    _name: Option<&str>,
    cache: bool,
) -> Response {
    let len = bytes.len();
    let mut response = (status, Body::from(bytes)).into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_str(mime).unwrap());
    response.headers_mut().insert(
        CONTENT_LENGTH,
        HeaderValue::from_str(&len.to_string()).unwrap(),
    );
    if cache {
        response.headers_mut().insert(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    }
    response
}

fn resize_image(bytes: Vec<u8>, size: u32) -> anyhow::Result<Vec<u8>> {
    let image = image::load_from_memory(&bytes)?;
    let resized = image.thumbnail(size, size);
    let mut out = Cursor::new(Vec::new());
    resized.write_to(&mut out, image::ImageFormat::Jpeg)?;
    Ok(out.into_inner())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestedRange {
    None,
    Valid(u64, u64),
    Invalid,
}

fn requested_range(headers: &HeaderMap, len: u64) -> RequestedRange {
    let Some(range) = headers.get(RANGE) else {
        return RequestedRange::None;
    };
    let Ok(range) = range.to_str() else {
        return RequestedRange::Invalid;
    };
    parse_range(range, len)
        .map(|(start, end)| RequestedRange::Valid(start, end))
        .unwrap_or(RequestedRange::Invalid)
}

fn parse_range(range: &str, len: u64) -> Option<(u64, u64)> {
    if len == 0 {
        return None;
    }
    let range = range.strip_prefix("bytes=")?;
    let (start, end) = range.split_once('-')?;
    if start.is_empty() {
        let suffix = end.parse::<u64>().ok()?;
        if suffix == 0 {
            return None;
        }
        let start = len.saturating_sub(suffix);
        return Some((start, len.saturating_sub(1)));
    }
    let start = start.parse::<u64>().ok()?;
    let end = if end.is_empty() {
        len.saturating_sub(1)
    } else {
        end.parse::<u64>().ok()?.min(len.saturating_sub(1))
    };
    (start <= end && end < len).then_some((start, end))
}

async fn resolve_opt(
    state: &AppState,
    kind: &str,
    value: Option<String>,
) -> ApiResult<Option<i64>> {
    match value {
        Some(value) if !value.trim().is_empty() => {
            Ok(Some(db::resolve_id(&state.pool, kind, &value).await?))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use easy_musiclib_media::path_hash;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::io::Write;
    use std::sync::Arc;

    #[test]
    fn hls_playlist_for_playback_prefers_zero_start_for_event_playlist() {
        let playlist = "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-PLAYLIST-TYPE:EVENT\n#EXT-X-MAP:URI=\"init.mp4\"\n#EXTINF:2.000000,\nsegment_00000.m4s\n";

        let rewritten = hls_playlist_for_playback(playlist);

        assert!(rewritten.contains(
            "#EXT-X-VERSION:7\n#EXT-X-START:TIME-OFFSET=0.000,PRECISE=YES\n#EXT-X-PLAYLIST-TYPE:EVENT"
        ));
    }

    #[test]
    fn hls_playlist_for_playback_keeps_existing_start_tag() {
        let playlist = "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-START:TIME-OFFSET=0.000,PRECISE=YES\n#EXT-X-PLAYLIST-TYPE:VOD\n";

        assert_eq!(hls_playlist_for_playback(playlist), playlist);
    }

    #[tokio::test]
    async fn fetch_track_detail_backfills_missing_file_duration() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::schema::init_db(&pool).await.unwrap();

        let temp = tempfile::tempdir().unwrap();
        let wav_path = temp.path().join("legacy.wav");
        write_test_wav(&wav_path, 2, 8_000);

        let metadata = std::fs::metadata(&wav_path).unwrap();
        let (media_file_id, _) = db::upsert_media_file(
            &pool,
            &wav_path.to_string_lossy(),
            &path_hash(&wav_path),
            metadata.len().try_into().unwrap(),
            0,
            "wav",
        )
        .await
        .unwrap();
        let track_id = db::insert_track(
            &pool,
            db::NewTrack {
                title: "Legacy WAV",
                album_id: None,
                event_id: None,
                cue_track_no: None,
                disc_no: None,
                track_no: None,
                duration_ms: None,
                date: None,
                year: None,
                artwork_id: None,
            },
            &[],
        )
        .await
        .unwrap();
        db::insert_track_audio_source(
            &pool,
            track_id,
            db::NewTrackAudioSource {
                kind: "file",
                media_file_id,
                cue_sheet_id: None,
                codec: "wav",
                sample_rate: None,
                start_sample: None,
                end_sample: None,
                start_ms: None,
                end_ms: None,
                renderer: PASSTHROUGH_RENDERER,
            },
        )
        .await
        .unwrap();

        let state = AppState {
            pool: pool.clone(),
            static_dir: Arc::new(temp.path().to_path_buf()),
        };
        let detail = fetch_track_detail_with_duration(&state, track_id)
            .await
            .unwrap();
        let duration_ms = detail.summary.duration_ms.unwrap();
        assert!((1_900..=2_100).contains(&duration_ms));

        let stored: i64 = sqlx::query_scalar("SELECT duration_ms FROM tracks WHERE id = ?")
            .bind(track_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stored, duration_ms);
    }

    fn write_test_wav(path: &std::path::Path, seconds: u32, sample_rate: u32) {
        let channels = 1_u16;
        let bits_per_sample = 16_u16;
        let samples = seconds * sample_rate;
        let data_len = samples * u32::from(channels) * u32::from(bits_per_sample / 8);
        let mut file = std::fs::File::create(path).unwrap();

        file.write_all(b"RIFF").unwrap();
        file.write_all(&(36 + data_len).to_le_bytes()).unwrap();
        file.write_all(b"WAVE").unwrap();
        file.write_all(b"fmt ").unwrap();
        file.write_all(&16_u32.to_le_bytes()).unwrap();
        file.write_all(&1_u16.to_le_bytes()).unwrap();
        file.write_all(&channels.to_le_bytes()).unwrap();
        file.write_all(&sample_rate.to_le_bytes()).unwrap();
        file.write_all(&(sample_rate * u32::from(channels) * 2).to_le_bytes())
            .unwrap();
        file.write_all(&(channels * 2).to_le_bytes()).unwrap();
        file.write_all(&bits_per_sample.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&data_len.to_le_bytes()).unwrap();

        for _ in 0..samples {
            file.write_all(&0_i16.to_le_bytes()).unwrap();
        }
    }
}
