use crate::db;
use crate::lyrics;
use crate::scanner;
use crate::{ApiResult, AppError, AppState};
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE,
    RANGE,
};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use easy_musiclib_media::render::{RenderTags, render_flac_cue_track, render_wav_slice};
use easy_musiclib_media::tags::read_embedded_picture;
use easy_musiclib_shared::*;
use serde::Deserialize;
use sqlx::Row;
use std::io::Cursor;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
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
    Ok(Json(db::fetch_track_detail(&state.pool, id).await?))
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
    Ok(Json(db::fetch_track_detail(&state.pool, id).await?))
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
        let detail = db::fetch_track_detail(&state.pool, id).await?;
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
                read_embedded_picture(
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
    headers: HeaderMap,
) -> ApiResult<Response> {
    render_track_response(state, id, headers, false).await
}

pub async fn download_track(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    render_track_response(state, id, headers, true).await
}

async fn render_track_response(
    state: AppState,
    id: String,
    headers: HeaderMap,
    download: bool,
) -> ApiResult<Response> {
    let id = db::resolve_id(&state.pool, "tracks", &id).await?;
    let source = db::track_render_source(&state.pool, id).await?;
    match source.renderer.as_str() {
        "passthrough" => passthrough_file(&source.path, &source.title, headers, download).await,
        "flac_tracksplit" => {
            let tags = RenderTags {
                title: source.title.clone(),
                artist: source.artist.clone(),
                album: source.album.clone(),
                track_no: source.track_no,
                date: source.date.clone(),
            };
            let path = PathBuf::from(source.path.clone());
            let bytes = tokio::task::spawn_blocking(move || {
                render_flac_cue_track(
                    &path,
                    source.start_sample.unwrap_or(0),
                    source.end_sample,
                    &tags,
                )
            })
            .await
            .map_err(|e| AppError::internal(e.to_string()))??;
            Ok(audio_response(bytes, "audio/flac", &source.title, download))
        }
        "wav_slice" => {
            let path = PathBuf::from(source.path.clone());
            let bytes = tokio::task::spawn_blocking(move || {
                render_wav_slice(&path, source.start_sample.unwrap_or(0), source.end_sample)
            })
            .await
            .map_err(|e| AppError::internal(e.to_string()))??;
            Ok(audio_response(bytes, "audio/wav", &source.title, download))
        }
        _ => Err(AppError::bad_request("track is not playable")),
    }
}

async fn passthrough_file(
    path: &str,
    title: &str,
    headers: HeaderMap,
    download: bool,
) -> ApiResult<Response> {
    let mut file = tokio::fs::File::open(path).await?;
    let len = file.metadata().await?.len();
    let mime = mime_guess::from_path(path)
        .first_raw()
        .unwrap_or("application/octet-stream")
        .to_string();
    let range = headers
        .get(RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| parse_range(s, len));
    let (status, start, end) = if let Some((start, end)) = range {
        (StatusCode::PARTIAL_CONTENT, start, end)
    } else {
        (StatusCode::OK, 0, len.saturating_sub(1))
    };
    let read_len = end.saturating_sub(start).saturating_add(1);
    file.seek(std::io::SeekFrom::Start(start)).await?;
    let stream = ReaderStream::new(file.take(read_len));
    let mut response = (status, Body::from_stream(stream)).into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_str(&mime).unwrap());
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
                "attachment; filename*=UTF-8''{}",
                percent_encoding::utf8_percent_encode(title, percent_encoding::NON_ALPHANUMERIC)
            ))
            .unwrap(),
        );
    }
    Ok(response)
}

fn audio_response(bytes: Vec<u8>, mime: &str, title: &str, download: bool) -> Response {
    let mut response = binary_response(StatusCode::OK, bytes, mime, Some(title), false);
    if download {
        response.headers_mut().insert(
            CONTENT_DISPOSITION,
            HeaderValue::from_str(&format!(
                "attachment; filename*=UTF-8''{}.{}",
                percent_encoding::utf8_percent_encode(title, percent_encoding::NON_ALPHANUMERIC),
                if mime == "audio/wav" { "wav" } else { "flac" }
            ))
            .unwrap(),
        );
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

fn parse_range(range: &str, len: u64) -> Option<(u64, u64)> {
    let range = range.strip_prefix("bytes=")?;
    let (start, end) = range.split_once('-')?;
    if start.is_empty() {
        let suffix = end.parse::<u64>().ok()?;
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
