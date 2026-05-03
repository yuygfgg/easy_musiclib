use crate::application::playback::{
    self as playback_app, BrowserAudioRequest, BrowserStreamPlan, HLS_INIT_FILE, HLS_MEDIA_MIME,
    HLS_PLAYLIST_FILE, HLS_PLAYLIST_MIME, PlaybackMedia,
};
use crate::application::settings as settings_app;
use crate::domain::{BrowserPlaybackSettings, PlaybackSource};
use crate::http::responses::{audio_bytes_response, ranged_file_response};
use crate::services::{hls_cache, track_duration};
use crate::{ApiResult, AppError, AppState};
use axum::body::Body;
use axum::http::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE,
};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::os::fd::IntoRawFd;
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;
use tokio::time::{Duration, Instant, sleep};
use tokio_util::io::ReaderStream;

pub async fn stream_track_response(
    state: AppState,
    id: String,
    requested_start_ms: i64,
    buffered: bool,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let id = playback_app::resolve_track_id(&state.repositories.playback, &id).await?;
    let source = playback_app::track_render_source(&state.repositories.playback, id).await?;
    if !state
        .services
        .playback_media
        .is_playable_renderer(Some(&source.renderer))
    {
        return Err(AppError::bad_request("track is not playable"));
    }
    track_duration::ensure_track_duration_ms(&state, id).await?;
    let source = playback_app::track_render_source(&state.repositories.playback, id).await?;
    let plan = playback_app::browser_stream_plan(
        source,
        settings_app::get_settings(&state.repositories.settings)
            .await?
            .browser_playback,
        requested_start_ms,
        buffered,
    );
    stream_browser_audio(state.services.playback_media, plan, headers).await
}

pub async fn stream_track_hls_file_response(
    state: AppState,
    id: String,
    file: String,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let id = playback_app::resolve_track_id(&state.repositories.playback, &id).await?;
    let source = playback_app::track_render_source(&state.repositories.playback, id).await?;
    if !state
        .services
        .playback_media
        .is_playable_renderer(Some(&source.renderer))
    {
        return Err(AppError::bad_request("track is not playable"));
    }
    let playback = settings_app::get_settings(&state.repositories.settings)
        .await?
        .browser_playback;
    let cache_dir = hls_cache::hls_cache_dir(id, &source, playback.flac_sample_rate).await?;
    let path = hls_cache::hls_file_path(&cache_dir, &file)?;
    hls_cache::ensure_hls_generation(
        state.services.playback_media.clone(),
        &source,
        &cache_dir,
        playback.flac_sample_rate,
    )
    .await?;
    let mime = hls_file_mime(&file)?;
    if file == HLS_PLAYLIST_FILE {
        hls_cache::wait_for_hls_playlist_start(&cache_dir, &path).await?;
        return hls_playlist_response(&path).await;
    }
    hls_cache::wait_for_hls_file(&path, hls_cache::hls_file_timeout(&file)).await?;
    let mut response = ranged_file_response(&path, mime, None, &headers, false).await?;
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    Ok(response)
}

pub async fn download_track_response(
    state: AppState,
    id: String,
    headers: HeaderMap,
) -> ApiResult<Response> {
    render_track_response(state, id, headers, true).await
}

pub async fn raw_track_response(
    state: AppState,
    id: String,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let id = playback_app::resolve_track_id(&state.repositories.playback, &id).await?;
    let source = playback_app::track_render_source(&state.repositories.playback, id).await?;
    match playback_app::track_render_plan(
        source,
        state.services.playback_media.passthrough_renderer(),
    ) {
        playback_app::TrackRenderPlan::PassthroughFile { path, title } => {
            passthrough_file(&path, &title, headers, false).await
        }
        playback_app::TrackRenderPlan::RenderedTrack { source } => {
            if !state
                .services
                .playback_media
                .is_exact_cue_renderer(Some(&source.renderer))
            {
                return Err(AppError::bad_request(
                    "raw playback is not available for track",
                ));
            }
            let format_info = state
                .services
                .playback_media
                .cue_audio_format(&source.renderer)
                .ok_or_else(|| AppError::bad_request("raw playback is not available for track"))?;
            let cache_dir = raw_cue_cache_dir(id, &source).await?;
            let path = cache_dir.join(format!("track.{}", format_info.extension));
            ensure_raw_cue_cache(state.services.playback_media, source, &path).await?;
            ranged_file_response(&path, format_info.mime, None, &headers, false).await
        }
    }
}

async fn render_track_response(
    state: AppState,
    id: String,
    headers: HeaderMap,
    download: bool,
) -> ApiResult<Response> {
    let id = playback_app::resolve_track_id(&state.repositories.playback, &id).await?;
    let source = playback_app::track_render_source(&state.repositories.playback, id).await?;
    match playback_app::track_render_plan(
        source,
        state.services.playback_media.passthrough_renderer(),
    ) {
        playback_app::TrackRenderPlan::PassthroughFile { path, title } => {
            passthrough_file(&path, &title, headers, download).await
        }
        playback_app::TrackRenderPlan::RenderedTrack { source } => {
            if !state
                .services
                .playback_media
                .is_playable_renderer(Some(&source.renderer))
            {
                return Err(AppError::bad_request("track is not playable"));
            }

            let rendered = state
                .services
                .playback_media
                .render_cue_track(&source)
                .await?;
            Ok(audio_bytes_response(
                rendered.bytes,
                rendered.mime,
                rendered.extension,
                &source.title,
                download,
                &headers,
            ))
        }
    }
}

async fn stream_browser_audio<M>(
    playback_media: M,
    plan: BrowserStreamPlan,
    headers: HeaderMap,
) -> ApiResult<Response>
where
    M: PlaybackMedia + Clone + Send + Sync + 'static,
{
    if plan.buffered {
        return buffered_browser_audio(
            &playback_media,
            plan.source,
            plan.playback,
            plan.absolute_start_ms,
            plan.end_ms,
            headers,
        )
        .await;
    }

    streaming_browser_audio(
        playback_media,
        plan.source,
        plan.playback,
        plan.absolute_start_ms,
        plan.end_ms,
    )
    .await
}

async fn buffered_browser_audio(
    playback_media: &impl PlaybackMedia,
    source: PlaybackSource,
    playback: BrowserPlaybackSettings,
    absolute_start_ms: i64,
    end_ms: Option<i64>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let title = source.title.clone();
    let rendered = playback_media
        .transcode_browser_audio(BrowserAudioRequest {
            path: PathBuf::from(source.path),
            playback,
            start_ms: absolute_start_ms,
            end_ms,
        })
        .await?;

    Ok(audio_bytes_response(
        rendered.bytes,
        rendered.mime,
        rendered.extension,
        &title,
        false,
        &headers,
    ))
}

async fn streaming_browser_audio<M>(
    playback_media: M,
    source: PlaybackSource,
    playback: BrowserPlaybackSettings,
    absolute_start_ms: i64,
    end_ms: Option<i64>,
) -> ApiResult<Response>
where
    M: PlaybackMedia + Clone + Send + Sync + 'static,
{
    let (reader, writer) = StdUnixStream::pair()?;
    reader.set_nonblocking(true)?;
    let reader = tokio::net::UnixStream::from_std(reader)?;
    let output_fd = writer.into_raw_fd();
    let input_path = PathBuf::from(source.path.clone());
    let title = source.title.clone();
    let format_info = playback_media.browser_audio_format(playback.clone());
    tokio::spawn(async move {
        if let Err(err) = playback_media
            .transcode_browser_audio_to_fd(
                BrowserAudioRequest {
                    path: input_path.clone(),
                    playback,
                    start_ms: absolute_start_ms,
                    end_ms,
                },
                output_fd,
            )
            .await
        {
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
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(format_info.mime));
    response
        .headers_mut()
        .insert(ACCEPT_RANGES, HeaderValue::from_static("none"));
    response.headers_mut().insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(
            "inline; filename*=UTF-8''{}.{}",
            percent_encoding::utf8_percent_encode(&title, percent_encoding::NON_ALPHANUMERIC),
            format_info.extension,
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

async fn raw_cue_cache_dir(
    track_id: crate::domain::TrackId,
    source: &PlaybackSource,
) -> ApiResult<PathBuf> {
    let metadata = tokio::fs::metadata(&source.path).await?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let mut hasher = Sha256::new();
    hasher.update(b"raw-cue-exact-v1");
    hasher.update(track_id.raw().to_le_bytes());
    hasher.update(source.path.as_bytes());
    hasher.update(source.renderer.as_bytes());
    hasher.update(source.codec.as_bytes());
    hasher.update(source.start_sample.unwrap_or(0).to_le_bytes());
    hasher.update(source.end_sample.unwrap_or(-1).to_le_bytes());
    hasher.update(source.start_ms.unwrap_or(0).to_le_bytes());
    hasher.update(source.end_ms.unwrap_or(-1).to_le_bytes());
    hasher.update(metadata.len().to_le_bytes());
    hasher.update(modified.to_le_bytes());
    Ok(raw_cue_cache_root().join(hex::encode(hasher.finalize())))
}

async fn ensure_raw_cue_cache<M>(
    playback_media: M,
    source: PlaybackSource,
    path: &Path,
) -> ApiResult<()>
where
    M: PlaybackMedia + Clone + Send + Sync + 'static,
{
    if raw_file_ready(path).await {
        return Ok(());
    }

    let path = path.to_path_buf();
    let should_generate = {
        let mut active = raw_cue_generators()
            .lock()
            .map_err(|_| AppError::internal("raw CUE generator lock poisoned"))?;
        active.insert(path.clone())
    };
    if !should_generate {
        return wait_for_raw_file(&path, Duration::from_secs(60)).await;
    }

    let result = generate_raw_cue_cache(playback_media, &source, &path).await;
    if let Ok(mut active) = raw_cue_generators().lock() {
        active.remove(&path);
    }
    result
}

async fn generate_raw_cue_cache<M>(
    playback_media: M,
    source: &PlaybackSource,
    path: &Path,
) -> ApiResult<()>
where
    M: PlaybackMedia + Clone + Send + Sync + 'static,
{
    if raw_file_ready(path).await {
        return Ok(());
    }
    let parent = path
        .parent()
        .ok_or_else(|| AppError::internal("raw CUE cache path has no parent"))?;
    tokio::fs::create_dir_all(parent).await?;
    let tmp_path = parent.join(format!(
        ".{}.tmp",
        path.file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_else(|| "track".into())
    ));
    if tokio::fs::metadata(&tmp_path).await.is_ok() {
        let _ = tokio::fs::remove_file(&tmp_path).await;
    }
    playback_media
        .render_cue_track_to_path(source, &tmp_path)
        .await?;
    tokio::fs::rename(&tmp_path, path).await?;
    Ok(())
}

async fn wait_for_raw_file(path: &Path, timeout: Duration) -> ApiResult<()> {
    let deadline = Instant::now() + timeout;
    loop {
        if raw_file_ready(path).await {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(AppError::not_found("raw audio is not ready"));
        }
        sleep(Duration::from_millis(50)).await;
    }
}

async fn raw_file_ready(path: &Path) -> bool {
    tokio::fs::metadata(path)
        .await
        .map(|metadata| metadata.is_file() && metadata.len() > 0)
        .unwrap_or(false)
}

fn raw_cue_cache_root() -> PathBuf {
    std::env::temp_dir().join("easy_musiclib_raw")
}

fn raw_cue_generators() -> &'static Mutex<HashSet<PathBuf>> {
    static ACTIVE: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    ACTIVE.get_or_init(|| Mutex::new(HashSet::new()))
}

fn hls_file_mime(file: &str) -> ApiResult<&'static str> {
    if file == HLS_PLAYLIST_FILE {
        Ok(HLS_PLAYLIST_MIME)
    } else if file == HLS_INIT_FILE || hls_cache::is_hls_segment_file(file) {
        Ok(HLS_MEDIA_MIME)
    } else {
        Err(AppError::not_found("HLS file not found"))
    }
}

async fn hls_playlist_response(path: &std::path::Path) -> ApiResult<Response> {
    let playlist = tokio::fs::read_to_string(path).await?;
    let playlist = hls_cache::hls_playlist_for_playback(&playlist);
    let len = playlist.len();
    let mut response = (StatusCode::OK, Body::from(playlist)).into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(HLS_PLAYLIST_MIME));
    response.headers_mut().insert(
        CONTENT_LENGTH,
        HeaderValue::from_str(&len.to_string()).unwrap(),
    );
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    Ok(response)
}
