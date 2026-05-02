use crate::application::playback::{
    self as playback_app, BrowserAudioRequest, BrowserStreamPlan, HLS_INIT_FILE, HLS_MEDIA_MIME,
    HLS_PLAYLIST_FILE, HLS_PLAYLIST_MIME, PlaybackMedia,
};
use crate::application::settings as settings_app;
use crate::domain::{BrowserPlaybackFormat, PlaybackSource};
use crate::http::responses::{audio_bytes_response, ranged_file_response};
use crate::services::{hls_cache, track_duration};
use crate::{ApiResult, AppError, AppState};
use axum::body::Body;
use axum::http::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE,
};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
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
            .browser_playback_format,
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
    let cache_dir = hls_cache::hls_cache_dir(id, &source).await?;
    let path = hls_cache::hls_file_path(&cache_dir, &file)?;
    hls_cache::ensure_hls_generation(state.services.playback_media.clone(), &source, &cache_dir)
        .await?;
    hls_cache::wait_for_hls_file(&path, hls_cache::hls_file_timeout(&file)).await?;
    let mime = hls_file_mime(&file)?;
    if file == HLS_PLAYLIST_FILE {
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
            plan.format,
            plan.absolute_start_ms,
            plan.end_ms,
            headers,
        )
        .await;
    }

    streaming_browser_audio(
        playback_media,
        plan.source,
        plan.format,
        plan.absolute_start_ms,
        plan.end_ms,
    )
    .await
}

async fn buffered_browser_audio(
    playback_media: &impl PlaybackMedia,
    source: PlaybackSource,
    playback_format: BrowserPlaybackFormat,
    absolute_start_ms: i64,
    end_ms: Option<i64>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let title = source.title.clone();
    let rendered = playback_media
        .transcode_browser_audio(BrowserAudioRequest {
            path: PathBuf::from(source.path),
            format: playback_format,
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
    playback_format: BrowserPlaybackFormat,
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
    let format_info = playback_media.browser_audio_format(playback_format);
    tokio::spawn(async move {
        if let Err(err) = playback_media
            .transcode_browser_audio_to_fd(
                BrowserAudioRequest {
                    path: input_path.clone(),
                    format: playback_format,
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
