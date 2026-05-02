use crate::cue_render::{CueTrackRenderer, WAV_SLICE_RENDERER};
use crate::formats::{AudioFormat, read_prefix};
use crate::render::{CueRenderQuality, RenderTags};
use anyhow::{Context, Result, bail};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

pub static FORMAT: Format = Format;

pub(crate) static CUE_RENDERER: CueRenderer = CueRenderer;

pub struct Format;

impl AudioFormat for Format {
    fn id(&self) -> &'static str {
        "wav"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["wav"]
    }

    fn mime(&self) -> Option<&'static str> {
        Some("audio/wav")
    }

    fn sniff(&self, path: &Path) -> Result<bool> {
        let mut buf = [0u8; 12];
        Ok(read_prefix(path, &mut buf)? == buf.len()
            && &buf[0..4] == b"RIFF"
            && &buf[8..12] == b"WAVE")
    }
}

pub(crate) struct CueRenderer;

impl CueTrackRenderer for CueRenderer {
    fn id(&self) -> &'static str {
        WAV_SLICE_RENDERER
    }

    fn priority(&self, format_id: &str) -> Option<i32> {
        (format_id == "wav").then_some(100)
    }

    fn output_mime(&self) -> &'static str {
        "audio/wav"
    }

    fn output_extension(&self) -> &'static str {
        "wav"
    }

    fn quality(&self) -> CueRenderQuality {
        CueRenderQuality::Lossless
    }

    fn render(
        &self,
        path: &Path,
        start_sample: i64,
        end_sample: Option<i64>,
        _tags: &RenderTags,
    ) -> Result<Vec<u8>> {
        render_slice(path, start_sample, end_sample)
    }
}

fn render_slice(path: &Path, start_sample: i64, end_sample: Option<i64>) -> Result<Vec<u8>> {
    let mut f = File::open(path).with_context(|| format!("opening wav {}", path.display()))?;
    let mut riff = [0u8; 12];
    f.read_exact(&mut riff)?;
    if &riff[0..4] != b"RIFF" || &riff[8..12] != b"WAVE" {
        bail!("not a RIFF/WAVE file");
    }

    let mut fmt_chunk = None;
    let mut data_chunk = None;
    loop {
        let mut header = [0u8; 8];
        if f.read_exact(&mut header).is_err() {
            break;
        }
        let id = [header[0], header[1], header[2], header[3]];
        let len = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as u64;
        let pos = f.stream_position()?;
        match &id {
            b"fmt " => {
                let mut bytes = vec![0u8; len as usize];
                f.read_exact(&mut bytes)?;
                fmt_chunk = Some(bytes);
            }
            b"data" => {
                data_chunk = Some((pos, len));
                f.seek(SeekFrom::Current(len as i64))?;
            }
            _ => {
                f.seek(SeekFrom::Current(len as i64))?;
            }
        }
        if len % 2 == 1 {
            f.seek(SeekFrom::Current(1))?;
        }
    }

    let fmt = fmt_chunk.context("wav fmt chunk missing")?;
    if fmt.len() < 16 {
        bail!("wav fmt chunk too short");
    }
    let channels = u16::from_le_bytes([fmt[2], fmt[3]]) as u64;
    let bits_per_sample = u16::from_le_bytes([fmt[14], fmt[15]]) as u64;
    let bytes_per_sample_frame = channels
        .checked_mul(bits_per_sample / 8)
        .filter(|v| *v > 0)
        .context("invalid wav channel or bit depth")?;
    let (data_pos, data_len) = data_chunk.context("wav data chunk missing")?;
    let total_frames = data_len / bytes_per_sample_frame;
    let start = (start_sample.max(0) as u64).min(total_frames);
    let end = end_sample
        .map(|s| s.max(start_sample) as u64)
        .unwrap_or(total_frames)
        .min(total_frames);
    let byte_start = data_pos + start * bytes_per_sample_frame;
    let byte_len = (end - start) * bytes_per_sample_frame;
    f.seek(SeekFrom::Start(byte_start))?;
    let mut data = vec![0u8; byte_len as usize];
    f.read_exact(&mut data)?;

    let riff_size = 4 + (8 + fmt.len() as u32) + (8 + data.len() as u32);
    let mut out = Vec::with_capacity(12 + 8 + fmt.len() + 8 + data.len());
    out.write_all(b"RIFF")?;
    out.write_all(&riff_size.to_le_bytes())?;
    out.write_all(b"WAVE")?;
    out.write_all(b"fmt ")?;
    out.write_all(&(fmt.len() as u32).to_le_bytes())?;
    out.write_all(&fmt)?;
    out.write_all(b"data")?;
    out.write_all(&(data.len() as u32).to_le_bytes())?;
    out.write_all(&data)?;
    Ok(out)
}
