use crate::domain::{BrowserPlaybackSettings, PlaybackSource, TrackId};
use anyhow::Result;
use futures::future::BoxFuture;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::fd::RawFd;

pub const HLS_PLAYLIST_FILE: &str = "playlist.m3u8";
pub const HLS_INIT_FILE: &str = "init.mp4";
pub const HLS_PLAYLIST_MIME: &str = "application/vnd.apple.mpegurl";
pub const HLS_MEDIA_MIME: &str = "audio/mp4";

pub trait PlaybackRepository: Send + Sync {
    fn resolve_track_id<'a>(&'a self, ident: &'a str) -> BoxFuture<'a, Result<TrackId>>;

    fn track_render_source(&self, track_id: TrackId) -> BoxFuture<'_, Result<PlaybackSource>>;
}

#[derive(Debug, Clone, Copy)]
pub struct BrowserAudioFormat {
    pub mime: &'static str,
    pub extension: &'static str,
}

#[derive(Debug)]
pub struct RenderedAudio {
    pub bytes: Vec<u8>,
    pub mime: &'static str,
    pub extension: &'static str,
}

#[derive(Debug, Clone)]
pub struct BrowserAudioRequest {
    pub path: PathBuf,
    pub playback: BrowserPlaybackSettings,
    pub start_ms: i64,
    pub end_ms: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct HlsRenderRequest {
    pub path: PathBuf,
    pub output_dir: PathBuf,
    pub start_ms: i64,
    pub end_ms: Option<i64>,
    pub flac_sample_rate: u32,
}

pub trait PlaybackMedia: Send + Sync {
    fn passthrough_renderer(&self) -> &'static str;

    fn browser_audio_format(&self, playback: BrowserPlaybackSettings) -> BrowserAudioFormat;

    fn is_playable_renderer(&self, renderer_id: Option<&str>) -> bool;

    fn is_exact_cue_renderer(&self, renderer_id: Option<&str>) -> bool;

    fn cue_audio_format(&self, renderer_id: &str) -> Option<BrowserAudioFormat>;

    fn render_cue_track<'a>(
        &'a self,
        source: &'a PlaybackSource,
    ) -> BoxFuture<'a, Result<RenderedAudio>>;

    fn render_cue_track_to_path<'a>(
        &'a self,
        source: &'a PlaybackSource,
        output_path: &'a Path,
    ) -> BoxFuture<'a, Result<()>>;

    fn transcode_browser_audio(
        &self,
        request: BrowserAudioRequest,
    ) -> BoxFuture<'static, Result<RenderedAudio>>;

    #[cfg(unix)]
    fn transcode_browser_audio_to_fd(
        &self,
        request: BrowserAudioRequest,
        output_fd: RawFd,
    ) -> BoxFuture<'static, Result<()>>;

    fn render_hls(&self, request: HlsRenderRequest) -> BoxFuture<'static, Result<()>>;
}

#[derive(Debug, Clone)]
pub struct BrowserStreamPlan {
    pub source: PlaybackSource,
    pub playback: BrowserPlaybackSettings,
    pub absolute_start_ms: i64,
    pub end_ms: Option<i64>,
    pub buffered: bool,
}

#[derive(Debug, Clone)]
pub enum TrackRenderPlan {
    PassthroughFile { path: String, title: String },
    RenderedTrack { source: PlaybackSource },
}

pub async fn resolve_track_id(
    repository: &impl PlaybackRepository,
    ident: &str,
) -> Result<TrackId> {
    repository.resolve_track_id(ident).await
}

pub async fn track_render_source(
    repository: &impl PlaybackRepository,
    track_id: TrackId,
) -> Result<PlaybackSource> {
    repository.track_render_source(track_id).await
}

pub fn browser_stream_plan(
    source: PlaybackSource,
    playback: BrowserPlaybackSettings,
    requested_start_ms: i64,
    buffered: bool,
) -> BrowserStreamPlan {
    let (absolute_start_ms, end_ms) = browser_stream_time_range(&source, requested_start_ms);
    BrowserStreamPlan {
        source,
        playback,
        absolute_start_ms,
        end_ms,
        buffered,
    }
}

pub fn track_render_plan(source: PlaybackSource, passthrough_renderer: &str) -> TrackRenderPlan {
    if source.renderer == passthrough_renderer {
        TrackRenderPlan::PassthroughFile {
            path: source.path,
            title: source.title,
        }
    } else {
        TrackRenderPlan::RenderedTrack { source }
    }
}

fn browser_stream_time_range(
    source: &PlaybackSource,
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

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PASSTHROUGH_RENDERER: &str = "passthrough";

    #[test]
    fn browser_stream_plan_clamps_relative_start_to_track_end() {
        let source = playback_source(Some(10_000), Some(20_000));
        let plan = browser_stream_plan(source, BrowserPlaybackSettings::default(), 50_000, false);

        assert_eq!(plan.absolute_start_ms, 19_999);
        assert_eq!(plan.end_ms, Some(20_000));
    }

    #[test]
    fn track_render_plan_passthrough_uses_original_file() {
        let source = playback_source(None, None);
        let plan = track_render_plan(source, TEST_PASSTHROUGH_RENDERER);

        match plan {
            TrackRenderPlan::PassthroughFile { path, title } => {
                assert_eq!(path, "song.flac");
                assert_eq!(title, "Song");
            }
            TrackRenderPlan::RenderedTrack { .. } => panic!("expected passthrough plan"),
        }
    }

    fn playback_source(start_ms: Option<i64>, end_ms: Option<i64>) -> PlaybackSource {
        PlaybackSource {
            title: "Song".to_string(),
            artist: "Artist".to_string(),
            album: None,
            track_no: None,
            date: None,
            path: "song.flac".to_string(),
            renderer: TEST_PASSTHROUGH_RENDERER.to_string(),
            codec: "flac".to_string(),
            start_sample: None,
            end_sample: None,
            start_ms,
            end_ms,
        }
    }
}
