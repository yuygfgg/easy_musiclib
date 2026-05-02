use crate::http::responses::{audio_bytes_response, ranged_file_response};
use crate::services::{hls_cache, track_duration};
use crate::{ApiResult, AppError, AppState, db};
use axum::body::Body;
use axum::http::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE,
};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use easy_musiclib_media::cue_render::{
    PASSTHROUGH_RENDERER, is_playable_renderer, render_cue_track_by_renderer,
};
use easy_musiclib_media::render::{
    FLAC_HLS_INIT_FILE, FLAC_HLS_MEDIA_MIME, FLAC_HLS_PLAYLIST_FILE, FLAC_HLS_PLAYLIST_MIME,
    PlaybackTranscodeFormat, RenderTags,
};
use easy_musiclib_media::transcode::{
    transcode_file_range_for_browser, transcode_file_range_for_browser_to_fd,
};
use easy_musiclib_shared::BrowserPlaybackFormat;
use std::os::fd::IntoRawFd;
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::PathBuf;
use tokio_util::io::ReaderStream;

pub async fn stream_track_response(
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
    track_duration::ensure_track_duration_ms(&state, id).await?;
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

pub async fn stream_track_hls_file_response(
    state: AppState,
    id: String,
    file: String,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let id = db::resolve_id(&state.pool, "tracks", &id).await?;
    let source = db::track_render_source(&state.pool, id).await?;
    if !is_playable_renderer(Some(&source.renderer)) {
        return Err(AppError::bad_request("track is not playable"));
    }
    let cache_dir = hls_cache::hls_cache_dir(id, &source).await?;
    let path = hls_cache::hls_file_path(&cache_dir, &file)?;
    hls_cache::ensure_hls_generation(&source, &cache_dir).await?;
    hls_cache::wait_for_hls_file(&path, hls_cache::hls_file_timeout(&file)).await?;
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

pub async fn download_track_response(
    state: AppState,
    id: String,
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

fn hls_file_mime(file: &str) -> ApiResult<&'static str> {
    if file == FLAC_HLS_PLAYLIST_FILE {
        Ok(FLAC_HLS_PLAYLIST_MIME)
    } else if file == FLAC_HLS_INIT_FILE || hls_cache::is_hls_segment_file(file) {
        Ok(FLAC_HLS_MEDIA_MIME)
    } else {
        Err(AppError::not_found("HLS file not found"))
    }
}

async fn hls_playlist_response(path: &std::path::Path) -> ApiResult<Response> {
    let playlist = tokio::fs::read_to_string(path).await?;
    let playlist = hls_cache::hls_playlist_for_playback(&playlist);
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
