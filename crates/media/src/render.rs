use crate::flac_split::render_flac_cue_track_exact;
use anyhow::{Context, Result, bail};
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct RenderTags {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub track_no: Option<i64>,
    pub date: Option<String>,
}

pub fn render_flac_cue_track(
    path: &Path,
    start_sample: i64,
    end_sample: Option<i64>,
    tags: &RenderTags,
) -> Result<Vec<u8>> {
    render_flac_cue_track_exact(path, start_sample, end_sample, tags)
}

pub fn render_wav_slice(
    path: &Path,
    start_sample: i64,
    end_sample: Option<i64>,
) -> Result<Vec<u8>> {
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

pub fn sniff_wav_sample_rate(path: &Path) -> Result<Option<i64>> {
    let mut f = File::open(path)?;
    let mut buf = [0u8; 44];
    let n = f.read(&mut buf)?;
    if n < 28 || &buf[0..4] != b"RIFF" || &buf[8..12] != b"WAVE" {
        return Ok(None);
    }
    Ok(Some(
        u32::from_le_bytes([buf[24], buf[25], buf[26], buf[27]]) as i64,
    ))
}

pub fn read_all_from_cursor(cursor: &mut Cursor<Vec<u8>>) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    cursor.read_to_end(&mut out)?;
    Ok(out)
}
