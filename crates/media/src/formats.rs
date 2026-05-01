use crate::render::{PlaybackTranscodeFormat, RenderTags, TranscodedAudio};
use crate::tags::{AudioTags, read_audio_tags, read_embedded_picture};
use crate::{ffmpeg_backend, flac, mp3, wav};
use anyhow::{Context, Result, anyhow};
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub const PASSTHROUGH_RENDERER: &str = "passthrough";
pub const UNSUPPORTED_CUE_RENDERER: &str = "unsupported_cue";
pub const FLAC_TRACKSPLIT_RENDERER: &str = "flac_tracksplit";
pub const WAV_SLICE_RENDERER: &str = "wav_slice";
pub const FFMPEG_CUE_RENDERER: &str = "ffmpeg_cue";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CueRenderQuality {
    Lossless,
    Lossy,
}

#[derive(Debug)]
pub struct RenderedCueTrack {
    pub bytes: Vec<u8>,
    pub mime: &'static str,
    pub extension: &'static str,
    pub quality: CueRenderQuality,
}

pub trait MetadataReader: Sync + Send {
    fn id(&self) -> &'static str;
    fn audio_tags_priority(&self, path: &Path, format_id: Option<&str>) -> Option<i32>;
    fn embedded_picture_priority(&self, path: &Path, format_id: Option<&str>) -> Option<i32>;
    fn read_audio_tags(&self, path: &Path, split_exceptions: &[String]) -> Result<AudioTags>;
    fn read_embedded_picture(&self, path: &Path, index: i64) -> Result<(Vec<u8>, Option<String>)>;
}

pub trait CueTrackRenderer: Sync + Send {
    fn id(&self) -> &'static str;
    fn priority(&self, format_id: &str) -> Option<i32>;
    fn output_mime(&self) -> &'static str;
    fn output_extension(&self) -> &'static str;
    fn quality(&self) -> CueRenderQuality;
    fn render(
        &self,
        path: &Path,
        start_sample: i64,
        end_sample: Option<i64>,
        tags: &RenderTags,
    ) -> Result<Vec<u8>>;
}

pub trait AudioFormat: Sync + Send {
    fn id(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];
    fn mime(&self) -> Option<&'static str>;

    fn matches_extension(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                self.extensions()
                    .iter()
                    .any(|supported| ext.eq_ignore_ascii_case(supported))
            })
            .unwrap_or(false)
    }

    fn sniff(&self, path: &Path) -> Result<bool> {
        Ok(self.matches_extension(path))
    }
}

pub static LOFTY_METADATA_READER: LoftyMetadataReader = LoftyMetadataReader;

pub struct LoftyMetadataReader;

impl MetadataReader for LoftyMetadataReader {
    fn id(&self) -> &'static str {
        "lofty"
    }

    fn audio_tags_priority(&self, _path: &Path, format_id: Option<&str>) -> Option<i32> {
        known_format_id(format_id).then_some(100)
    }

    fn embedded_picture_priority(&self, _path: &Path, format_id: Option<&str>) -> Option<i32> {
        known_format_id(format_id).then_some(100)
    }

    fn read_audio_tags(&self, path: &Path, split_exceptions: &[String]) -> Result<AudioTags> {
        read_audio_tags(path, split_exceptions)
    }

    fn read_embedded_picture(&self, path: &Path, index: i64) -> Result<(Vec<u8>, Option<String>)> {
        read_embedded_picture(path, index)
    }
}

pub fn audio_formats() -> &'static [&'static dyn AudioFormat] {
    &AUDIO_FORMATS
}

pub fn metadata_readers() -> &'static [&'static dyn MetadataReader] {
    &METADATA_READERS
}

pub fn cue_renderers() -> &'static [&'static dyn CueTrackRenderer] {
    &CUE_RENDERERS
}

pub fn format_by_id(id: &str) -> Option<&'static dyn AudioFormat> {
    audio_formats()
        .iter()
        .copied()
        .find(|format| format.id().eq_ignore_ascii_case(id))
}

pub fn format_by_extension(path: &Path) -> Option<&'static dyn AudioFormat> {
    audio_formats()
        .iter()
        .copied()
        .find(|format| format.matches_extension(path))
}

pub fn detect_format(path: &Path) -> Result<Option<&'static dyn AudioFormat>> {
    if let Some(format) = format_by_extension(path) {
        return Ok(Some(format));
    }
    sniff_format(path)
}

pub fn sniff_format(path: &Path) -> Result<Option<&'static dyn AudioFormat>> {
    for format in audio_formats() {
        if format.sniff(path)? {
            return Ok(Some(*format));
        }
    }
    Ok(None)
}

pub fn read_audio_metadata(path: &Path, split_exceptions: &[String]) -> Result<AudioTags> {
    let format_id = format_by_extension(path).map(|format| format.id());
    let reader = best_metadata_reader(|reader| reader.audio_tags_priority(path, format_id))
        .ok_or_else(|| anyhow!("no metadata reader available for {}", path.display()))?;
    reader
        .read_audio_tags(path, split_exceptions)
        .with_context(|| {
            format!(
                "metadata reader {} failed for {}",
                reader.id(),
                path.display()
            )
        })
}

pub fn read_embedded_picture_for_path(
    path: &Path,
    index: i64,
) -> Result<(Vec<u8>, Option<String>)> {
    let format_id = format_by_extension(path).map(|format| format.id());
    let reader = best_metadata_reader(|reader| reader.embedded_picture_priority(path, format_id))
        .ok_or_else(|| {
        anyhow!(
            "no embedded picture reader available for {}",
            path.display()
        )
    })?;
    reader.read_embedded_picture(path, index).with_context(|| {
        format!(
            "picture reader {} failed for {}",
            reader.id(),
            path.display()
        )
    })
}

pub fn cue_renderer_id_for_format_id(format_id: &str) -> &'static str {
    cue_renderer_for_format_id(format_id)
        .map(|renderer| renderer.id())
        .unwrap_or(UNSUPPORTED_CUE_RENDERER)
}

pub fn cue_renderer_for_format_id(format_id: &str) -> Option<&'static dyn CueTrackRenderer> {
    best_cue_renderer(|renderer| renderer.priority(format_id))
}

pub fn cue_renderer_by_id(renderer_id: &str) -> Option<&'static dyn CueTrackRenderer> {
    cue_renderers()
        .iter()
        .copied()
        .find(|renderer| renderer.id() == renderer_id)
}

pub fn is_playable_renderer(renderer_id: Option<&str>) -> bool {
    match renderer_id {
        Some(PASSTHROUGH_RENDERER) => true,
        Some(renderer_id) => cue_renderer_by_id(renderer_id).is_some(),
        None => false,
    }
}

pub fn render_cue_track_by_renderer(
    renderer_id: &str,
    path: &Path,
    start_sample: i64,
    end_sample: Option<i64>,
    tags: &RenderTags,
) -> Result<RenderedCueTrack> {
    let renderer = cue_renderer_by_id(renderer_id)
        .ok_or_else(|| anyhow!("unsupported cue renderer: {renderer_id}"))?;
    Ok(RenderedCueTrack {
        bytes: renderer.render(path, start_sample, end_sample, tags)?,
        mime: renderer.output_mime(),
        extension: renderer.output_extension(),
        quality: renderer.quality(),
    })
}

pub fn transcode_file_for_browser(
    path: &Path,
    format: PlaybackTranscodeFormat,
) -> Result<TranscodedAudio> {
    ffmpeg_backend::transcode_file_for_browser(path, format)
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

pub(crate) fn read_prefix(path: &Path, buf: &mut [u8]) -> Result<usize> {
    let mut file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    file.read(buf)
        .with_context(|| format!("reading {}", path.display()))
}

static AUDIO_FORMATS: [&'static dyn AudioFormat; 3] = [&flac::FORMAT, &mp3::FORMAT, &wav::FORMAT];
static METADATA_READERS: [&'static dyn MetadataReader; 1] = [&LOFTY_METADATA_READER];
static CUE_RENDERERS: [&'static dyn CueTrackRenderer; 3] = [
    &flac::CUE_RENDERER,
    &wav::CUE_RENDERER,
    &ffmpeg_backend::CUE_RENDERER,
];

fn known_format_id(format_id: Option<&str>) -> bool {
    format_id.and_then(format_by_id).is_some()
}

fn best_metadata_reader(
    priority: impl Fn(&'static dyn MetadataReader) -> Option<i32>,
) -> Option<&'static dyn MetadataReader> {
    best_by_priority(metadata_readers(), priority)
}

fn best_cue_renderer(
    priority: impl Fn(&'static dyn CueTrackRenderer) -> Option<i32>,
) -> Option<&'static dyn CueTrackRenderer> {
    best_by_priority(cue_renderers(), priority)
}

fn best_by_priority<T: ?Sized>(
    items: &'static [&'static T],
    priority: impl Fn(&'static T) -> Option<i32>,
) -> Option<&'static T> {
    items
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(idx, item)| priority(item).map(|score| (score, std::cmp::Reverse(idx), item)))
        .max_by_key(|(score, reverse_idx, _)| (*score, *reverse_idx))
        .map(|(_, _, item)| item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn maps_existing_extensions_to_formats() {
        assert_eq!(
            format_by_extension(Path::new("a.flac")).map(|format| format.id()),
            Some("flac")
        );
        assert_eq!(
            format_by_extension(Path::new("a.MP3")).map(|format| format.id()),
            Some("mp3")
        );
        assert_eq!(
            format_by_extension(Path::new("a.wav")).map(|format| format.id()),
            Some("wav")
        );
        assert!(format_by_extension(Path::new("a.cue")).is_none());
        assert!(format_by_extension(Path::new("a.ogg")).is_none());
    }

    #[test]
    fn exposes_current_cue_renderers() {
        assert_eq!(
            cue_renderer_id_for_format_id("flac"),
            FLAC_TRACKSPLIT_RENDERER
        );
        assert_eq!(cue_renderer_id_for_format_id("wav"), WAV_SLICE_RENDERER);
        assert_eq!(
            cue_renderer_id_for_format_id("mp3"),
            crate::formats::FFMPEG_CUE_RENDERER
        );
        assert!(is_playable_renderer(Some(PASSTHROUGH_RENDERER)));
        assert!(is_playable_renderer(Some(FLAC_TRACKSPLIT_RENDERER)));
        assert!(!is_playable_renderer(Some(UNSUPPORTED_CUE_RENDERER)));
    }

    #[test]
    fn priority_ties_keep_registration_order() {
        assert_eq!(
            best_by_priority(&[&1_i32, &2_i32], |_| Some(10)).copied(),
            Some(1)
        );
    }
}
