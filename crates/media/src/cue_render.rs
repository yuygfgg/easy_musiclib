use crate::render::{CueRenderQuality, RenderTags};
use crate::{ffmpeg_backend, flac, wav};
use anyhow::{Result, anyhow};
use std::path::Path;

pub const PASSTHROUGH_RENDERER: &str = "passthrough";
pub const UNSUPPORTED_CUE_RENDERER: &str = "unsupported_cue";
pub const FLAC_TRACKSPLIT_RENDERER: &str = "flac_tracksplit";
pub const WAV_SLICE_RENDERER: &str = "wav_slice";
pub const FFMPEG_CUE_RENDERER: &str = "ffmpeg_cue";

#[derive(Debug)]
pub struct RenderedCueTrack {
    pub bytes: Vec<u8>,
    pub mime: &'static str,
    pub extension: &'static str,
    pub quality: CueRenderQuality,
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

pub fn cue_renderers() -> &'static [&'static dyn CueTrackRenderer] {
    &CUE_RENDERERS
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

static CUE_RENDERERS: [&'static dyn CueTrackRenderer; 3] = [
    &flac::CUE_RENDERER,
    &wav::CUE_RENDERER,
    &ffmpeg_backend::CUE_RENDERER,
];

fn best_cue_renderer(
    priority: impl Fn(&'static dyn CueTrackRenderer) -> Option<i32>,
) -> Option<&'static dyn CueTrackRenderer> {
    crate::formats::best_by_priority(cue_renderers(), priority)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_current_cue_renderers() {
        assert_eq!(
            cue_renderer_id_for_format_id("flac"),
            FLAC_TRACKSPLIT_RENDERER
        );
        assert_eq!(cue_renderer_id_for_format_id("wav"), WAV_SLICE_RENDERER);
        assert_eq!(cue_renderer_id_for_format_id("mp3"), FFMPEG_CUE_RENDERER);
        assert!(is_playable_renderer(Some(PASSTHROUGH_RENDERER)));
        assert!(is_playable_renderer(Some(FLAC_TRACKSPLIT_RENDERER)));
        assert!(!is_playable_renderer(Some(UNSUPPORTED_CUE_RENDERER)));
    }
}
