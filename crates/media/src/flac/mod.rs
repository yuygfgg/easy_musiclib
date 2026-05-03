mod split;

use crate::cue_render::{CueRenderKind, CueTrackRenderer, FLAC_TRACKSPLIT_RENDERER};
use crate::formats::{AudioFormat, read_prefix};
use crate::render::{CueRenderQuality, RenderTags};
use anyhow::Result;
use std::path::Path;

pub static FORMAT: Format = Format;

pub(crate) static CUE_RENDERER: CueRenderer = CueRenderer;

pub struct Format;

impl AudioFormat for Format {
    fn id(&self) -> &'static str {
        "flac"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["flac"]
    }

    fn mime(&self) -> Option<&'static str> {
        Some("audio/flac")
    }

    fn sniff(&self, path: &Path) -> Result<bool> {
        let mut buf = [0u8; 4];
        Ok(read_prefix(path, &mut buf)? == buf.len() && &buf == b"fLaC")
    }
}

pub(crate) struct CueRenderer;

impl CueTrackRenderer for CueRenderer {
    fn id(&self) -> &'static str {
        FLAC_TRACKSPLIT_RENDERER
    }

    fn priority(&self, format_id: &str) -> Option<i32> {
        (format_id == "flac").then_some(100)
    }

    fn kind(&self) -> CueRenderKind {
        CueRenderKind::ExactSlice
    }

    fn output_mime(&self) -> &'static str {
        "audio/flac"
    }

    fn output_extension(&self) -> &'static str {
        "flac"
    }

    fn quality(&self) -> CueRenderQuality {
        CueRenderQuality::Lossless
    }

    fn render(
        &self,
        path: &Path,
        start_sample: i64,
        end_sample: Option<i64>,
        tags: &RenderTags,
    ) -> Result<Vec<u8>> {
        split::render_cue_track_exact(path, start_sample, end_sample, tags)
    }
}
