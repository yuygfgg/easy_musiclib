use crate::extract_year;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CueSheet {
    pub path: PathBuf,
    pub audio_path: PathBuf,
    pub album_title: Option<String>,
    pub performer: Option<String>,
    pub date: Option<String>,
    pub tracks: Vec<CueTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CueTrack {
    pub no: i64,
    pub title: Option<String>,
    pub performer: Option<String>,
    pub start_frames: i64,
    pub start_ms: i64,
    pub end_ms: Option<i64>,
    pub start_sample: Option<i64>,
    pub end_sample: Option<i64>,
}

pub fn parse_cue_file(path: &Path) -> Result<CueSheet> {
    let bytes = std::fs::read(path).with_context(|| format!("reading cue {}", path.display()))?;
    let text = if let Ok(s) = std::str::from_utf8(&bytes) {
        s.to_string()
    } else {
        let mut detector = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Allow);
        detector.feed(&bytes, true);
        let encoding = detector.guess(None, chardetng::Utf8Detection::Allow);
        let (decoded, _, _) = encoding.decode(&bytes);
        decoded.into_owned()
    };
    parse_cue_text(path, &text)
}

pub fn parse_cue_text(path: &Path, text: &str) -> Result<CueSheet> {
    let mut album_title = None;
    let mut performer = None;
    let mut date = None;
    let mut audio_file: Option<PathBuf> = None;
    let mut tracks = Vec::new();
    let mut current: Option<CueTrack> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let (cmd, rest) = split_cmd(line);
        match cmd.to_ascii_uppercase().as_str() {
            "REM" => {
                let (key, value) = split_cmd(rest);
                if key.eq_ignore_ascii_case("DATE") && !value.trim().is_empty() {
                    date = Some(unquote(value).to_string());
                }
            }
            "FILE" => {
                let value = first_quoted_or_token(rest);
                let base = path.parent().unwrap_or_else(|| Path::new("."));
                audio_file = Some(base.join(value));
            }
            "TITLE" => {
                let value = unquote(rest).trim();
                if let Some(track) = current.as_mut() {
                    track.title = Some(value.to_string());
                } else {
                    album_title = Some(value.to_string());
                }
            }
            "PERFORMER" => {
                let value = unquote(rest).trim();
                if let Some(track) = current.as_mut() {
                    track.performer = Some(value.to_string());
                } else {
                    performer = Some(value.to_string());
                }
            }
            "TRACK" => {
                if let Some(track) = current.take() {
                    tracks.push(track);
                }
                let no = rest
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or((tracks.len() + 1) as i64);
                current = Some(CueTrack {
                    no,
                    title: None,
                    performer: None,
                    start_frames: 0,
                    start_ms: 0,
                    end_ms: None,
                    start_sample: None,
                    end_sample: None,
                });
            }
            "INDEX" => {
                let mut parts = rest.split_whitespace();
                let index_no = parts.next().unwrap_or_default();
                let timestamp = parts.next().unwrap_or_default();
                if index_no == "01" {
                    if let Some(track) = current.as_mut() {
                        let frames = parse_cue_timestamp(timestamp)?;
                        track.start_frames = frames;
                        track.start_ms = frames * 1000 / 75;
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(track) = current.take() {
        tracks.push(track);
    }

    let Some(audio_path) = audio_file else {
        bail!("cue has no FILE line: {}", path.display());
    };
    if tracks.is_empty() {
        bail!("cue has no tracks: {}", path.display());
    }
    Ok(CueSheet {
        path: path.to_path_buf(),
        audio_path,
        album_title,
        performer,
        date,
        tracks,
    })
}

pub fn apply_audio_timing(
    sheet: &mut CueSheet,
    sample_rate: Option<i64>,
    duration_ms: Option<i64>,
) {
    let sample_rate = sample_rate.unwrap_or(44_100);
    for idx in 0..sheet.tracks.len() {
        let start_frames = sheet.tracks[idx].start_frames;
        let end_ms = sheet
            .tracks
            .get(idx + 1)
            .map(|next| next.start_ms)
            .or(duration_ms);
        let end_frames = sheet
            .tracks
            .get(idx + 1)
            .map(|next| next.start_frames)
            .or_else(|| duration_ms.map(|ms| ms * 75 / 1000));

        sheet.tracks[idx].start_sample = Some(start_frames * sample_rate / 75);
        sheet.tracks[idx].end_ms = end_ms;
        sheet.tracks[idx].end_sample = end_frames.map(|frames| frames * sample_rate / 75);
    }
}

pub fn cue_year(sheet: &CueSheet) -> Option<i64> {
    extract_year(sheet.date.as_deref())
}

fn split_cmd(line: &str) -> (&str, &str) {
    let trimmed = line.trim();
    match trimmed.find(char::is_whitespace) {
        Some(idx) => (&trimmed[..idx], trimmed[idx..].trim()),
        None => (trimmed, ""),
    }
}

fn unquote(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn first_quoted_or_token(s: &str) -> &str {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            return &rest[..end];
        }
    }
    s.split_whitespace().next().unwrap_or(s)
}

fn parse_cue_timestamp(s: &str) -> Result<i64> {
    let mut parts = s.split(':');
    let mm = parts
        .next()
        .context("cue timestamp missing minutes")?
        .parse::<i64>()?;
    let ss = parts
        .next()
        .context("cue timestamp missing seconds")?
        .parse::<i64>()?;
    let ff = parts
        .next()
        .context("cue timestamp missing frames")?
        .parse::<i64>()?;
    Ok((mm * 60 + ss) * 75 + ff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_cue() {
        let sheet = parse_cue_text(
            Path::new("/music/a/test.cue"),
            r#"
REM DATE 2020
PERFORMER "Artist"
TITLE "Album"
FILE "disc.flac" WAVE
  TRACK 01 AUDIO
    TITLE "One"
    PERFORMER "Singer"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "Two"
    INDEX 01 03:10:37
"#,
        )
        .unwrap();
        assert_eq!(sheet.audio_path, PathBuf::from("/music/a/disc.flac"));
        assert_eq!(sheet.tracks.len(), 2);
        assert_eq!(
            sheet.tracks[1].start_ms,
            ((3 * 60 + 10) * 75 + 37) * 1000 / 75
        );
    }
}
