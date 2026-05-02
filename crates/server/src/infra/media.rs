use crate::application::playback::{
    BrowserAudioFormat, BrowserAudioRequest, HlsRenderRequest, PlaybackMedia, RenderedAudio,
};
use crate::application::scan::{
    ArtistNameParser, AudioMetadataReader, AudioTags, CueRendererSelector, CueSheet,
    CueSheetReader, CueTrack, DiscoveredAudioFile, DiscoveredCueFile, DiscoveredLibraryFiles,
    EmbeddedPictureInfo, LibraryFileDiscovery,
};
use crate::domain::{BrowserPlaybackFormat, PlaybackSource};
use anyhow::Result;
use easy_musiclib_media::cue as media_cue;
use easy_musiclib_media::cue_render;
use easy_musiclib_media::metadata::read_audio_metadata;
use easy_musiclib_media::providers as media_providers;
use easy_musiclib_media::render::{PlaybackTranscodeFormat, RenderTags};
use easy_musiclib_media::tags as media_tags;
use easy_musiclib_media::transcode;
use futures::FutureExt;
use futures::future::BoxFuture;
use std::path::Path;

#[cfg(unix)]
use std::os::fd::RawFd;

#[derive(Debug, Clone, Default)]
pub struct FilesystemLibraryFileDiscovery;

impl LibraryFileDiscovery for FilesystemLibraryFileDiscovery {
    fn discover_library_files<'a>(
        &'a self,
        roots: &'a [String],
    ) -> BoxFuture<'a, Result<DiscoveredLibraryFiles>> {
        async move {
            let roots = roots.to_vec();
            let discovered = tokio::task::spawn_blocking(move || {
                media_providers::discover_library_files(&roots)
            })
            .await??;
            Ok(discovered.into())
        }
        .boxed()
    }
}

#[derive(Debug, Clone, Default)]
pub struct LoftyAudioMetadataReader;

impl AudioMetadataReader for LoftyAudioMetadataReader {
    fn read_audio_metadata<'a>(
        &'a self,
        path: &'a Path,
        split_exceptions: &'a [String],
    ) -> BoxFuture<'a, Result<AudioTags>> {
        async move {
            let path = path.to_path_buf();
            let split_exceptions = split_exceptions.to_vec();
            tokio::task::spawn_blocking(move || read_audio_metadata(&path, &split_exceptions))
                .await?
                .map(Into::into)
        }
        .boxed()
    }
}

#[derive(Debug, Clone, Default)]
pub struct FilesystemCueSheetReader;

impl CueSheetReader for FilesystemCueSheetReader {
    fn parse_cue_file<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, Result<CueSheet>> {
        async move {
            let path = path.to_path_buf();
            let sheet =
                tokio::task::spawn_blocking(move || media_cue::parse_cue_file(&path)).await??;
            Ok(sheet.into())
        }
        .boxed()
    }
}

#[derive(Debug, Clone, Default)]
pub struct StaticCueRendererSelector;

impl CueRendererSelector for StaticCueRendererSelector {
    fn passthrough_renderer(&self) -> &'static str {
        cue_render::PASSTHROUGH_RENDERER
    }

    fn cue_renderer_id_for_format_id(&self, format_id: &str) -> &'static str {
        cue_render::cue_renderer_id_for_format_id(format_id)
    }
}

#[derive(Debug, Clone, Default)]
pub struct MediaArtistNameParser;

impl ArtistNameParser for MediaArtistNameParser {
    fn parse_artists(&self, raw: &[String], split_exceptions: &[String]) -> Vec<String> {
        easy_musiclib_media::artists::parse_artists(raw, split_exceptions)
    }
}

#[derive(Debug, Clone, Default)]
pub struct FfmpegPlaybackMedia;

impl PlaybackMedia for FfmpegPlaybackMedia {
    fn passthrough_renderer(&self) -> &'static str {
        cue_render::PASSTHROUGH_RENDERER
    }

    fn browser_audio_format(&self, format: BrowserPlaybackFormat) -> BrowserAudioFormat {
        let format = media_playback_format(format);
        BrowserAudioFormat {
            mime: format.mime(),
            extension: format.extension(),
        }
    }

    fn is_playable_renderer(&self, renderer_id: Option<&str>) -> bool {
        cue_render::is_playable_renderer(renderer_id)
    }

    fn render_cue_track<'a>(
        &'a self,
        source: &'a PlaybackSource,
    ) -> BoxFuture<'a, Result<RenderedAudio>> {
        async move {
            let tags = RenderTags {
                title: source.title.clone(),
                artist: source.artist.clone(),
                album: source.album.clone(),
                track_no: source.track_no,
                date: source.date.clone(),
            };
            let renderer = source.renderer.clone();
            let path = std::path::PathBuf::from(source.path.clone());
            let start_sample = source.start_sample.unwrap_or(0);
            let end_sample = source.end_sample;
            let rendered = tokio::task::spawn_blocking(move || {
                cue_render::render_cue_track_by_renderer(
                    &renderer,
                    &path,
                    start_sample,
                    end_sample,
                    &tags,
                )
            })
            .await??;
            Ok(RenderedAudio {
                bytes: rendered.bytes,
                mime: rendered.mime,
                extension: rendered.extension,
            })
        }
        .boxed()
    }

    fn transcode_browser_audio(
        &self,
        request: BrowserAudioRequest,
    ) -> BoxFuture<'static, Result<RenderedAudio>> {
        async move {
            let format = media_playback_format(request.format);
            let rendered = tokio::task::spawn_blocking(move || {
                transcode::transcode_file_range_for_browser(
                    &request.path,
                    format,
                    request.start_ms,
                    request.end_ms,
                )
            })
            .await??;
            Ok(RenderedAudio {
                bytes: rendered.bytes,
                mime: rendered.mime,
                extension: rendered.extension,
            })
        }
        .boxed()
    }

    #[cfg(unix)]
    fn transcode_browser_audio_to_fd(
        &self,
        request: BrowserAudioRequest,
        output_fd: RawFd,
    ) -> BoxFuture<'static, Result<()>> {
        async move {
            let format = media_playback_format(request.format);
            tokio::task::spawn_blocking(move || {
                transcode::transcode_file_range_for_browser_to_fd(
                    &request.path,
                    format,
                    request.start_ms,
                    request.end_ms,
                    output_fd,
                )
            })
            .await??;
            Ok(())
        }
        .boxed()
    }

    fn render_hls(&self, request: HlsRenderRequest) -> BoxFuture<'static, Result<()>> {
        async move {
            tokio::task::spawn_blocking(move || {
                transcode::render_flac_48k_hls(
                    &request.path,
                    &request.output_dir,
                    request.start_ms,
                    request.end_ms,
                )
            })
            .await??;
            Ok(())
        }
        .boxed()
    }
}

fn media_playback_format(format: BrowserPlaybackFormat) -> PlaybackTranscodeFormat {
    match format {
        BrowserPlaybackFormat::Opus256k => PlaybackTranscodeFormat::Opus256k,
        BrowserPlaybackFormat::Flac48k => PlaybackTranscodeFormat::Flac48k,
    }
}

impl From<media_providers::DiscoveredLibraryFiles> for DiscoveredLibraryFiles {
    fn from(value: media_providers::DiscoveredLibraryFiles) -> Self {
        Self {
            audio: value.audio.into_iter().map(Into::into).collect(),
            cues: value.cues.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<media_providers::DiscoveredAudioFile> for DiscoveredAudioFile {
    fn from(value: media_providers::DiscoveredAudioFile) -> Self {
        Self {
            path: value.path,
            path_hash: value.path_hash,
            size: value.size,
            mtime_ns: value.mtime_ns,
            format: value.format,
        }
    }
}

impl From<media_providers::DiscoveredCueFile> for DiscoveredCueFile {
    fn from(value: media_providers::DiscoveredCueFile) -> Self {
        Self {
            path: value.path,
            path_hash: value.path_hash,
            size: value.size,
            mtime_ns: value.mtime_ns,
        }
    }
}

impl From<media_tags::AudioTags> for AudioTags {
    fn from(value: media_tags::AudioTags) -> Self {
        Self {
            title: value.title,
            album: value.album,
            artists: value.artists,
            album_artists: value.album_artists,
            raw_artists: value.raw_artists,
            raw_album_artists: value.raw_album_artists,
            track_number: value.track_number,
            disc_number: value.disc_number,
            date: value.date,
            year: value.year,
            event: value.event,
            duration_ms: value.duration_ms,
            sample_rate: value.sample_rate,
            channels: value.channels,
            embedded_picture: value.embedded_picture.map(Into::into),
            sidecar_artwork: value.sidecar_artwork,
            format: value.format,
        }
    }
}

impl From<media_tags::EmbeddedPictureInfo> for EmbeddedPictureInfo {
    fn from(value: media_tags::EmbeddedPictureInfo) -> Self {
        Self {
            index: value.index,
            mime: value.mime,
        }
    }
}

impl From<media_cue::CueSheet> for CueSheet {
    fn from(value: media_cue::CueSheet) -> Self {
        Self {
            path: value.path,
            audio_path: value.audio_path,
            album_title: value.album_title,
            performer: value.performer,
            date: value.date,
            tracks: value.tracks.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<media_cue::CueTrack> for CueTrack {
    fn from(value: media_cue::CueTrack) -> Self {
        Self {
            no: value.no,
            title: value.title,
            performer: value.performer,
            start_frames: value.start_frames,
            start_ms: value.start_ms,
            end_ms: value.end_ms,
            start_sample: value.start_sample,
            end_sample: value.end_sample,
        }
    }
}
