use crate::cue_render::RenderedCueTrack;
use crate::ffmpeg_backend;
use crate::render::{PlaybackTranscodeFormat, TranscodedAudio};
use anyhow::Result;
use std::path::Path;

pub fn transcode_file_for_browser(
    path: &Path,
    format: PlaybackTranscodeFormat,
) -> Result<TranscodedAudio> {
    ffmpeg_backend::transcode_file_for_browser(path, format)
}

pub fn transcode_file_range_for_browser(
    path: &Path,
    format: PlaybackTranscodeFormat,
    start_ms: i64,
    end_ms: Option<i64>,
) -> Result<TranscodedAudio> {
    ffmpeg_backend::transcode_file_range_for_browser(path, format, start_ms, end_ms)
}

#[cfg(unix)]
pub fn transcode_file_range_for_browser_to_fd(
    path: &Path,
    format: PlaybackTranscodeFormat,
    start_ms: i64,
    end_ms: Option<i64>,
    output_fd: std::os::fd::RawFd,
) -> Result<()> {
    ffmpeg_backend::transcode_file_range_for_browser_to_fd(
        path, format, start_ms, end_ms, output_fd,
    )
}

pub fn transcode_rendered_cue_for_browser(
    rendered: RenderedCueTrack,
    format: PlaybackTranscodeFormat,
) -> Result<TranscodedAudio> {
    ffmpeg_backend::transcode_bytes_for_browser(rendered.bytes, rendered.extension, format)
}

pub fn render_flac_48k_hls(
    path: &Path,
    output_dir: &Path,
    start_ms: i64,
    end_ms: Option<i64>,
) -> Result<()> {
    ffmpeg_backend::render_flac_48k_hls(path, output_dir, start_ms, end_ms)
}
