use crate::api::api_get;
use easy_musiclib_macros::spawn_result;
use easy_musiclib_shared::{LyricsCandidate, TrackSummary};
use leptos::prelude::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub(crate) struct LyricLine {
    pub(crate) time_ms: i64,
    pub(crate) text: String,
    pub(crate) translations: Vec<String>,
}

#[derive(Clone, Debug)]
struct ParsedLyrics {
    lines: Vec<LyricLine>,
    plain: Vec<String>,
}

pub(crate) fn load_lyrics_for_track(
    track: TrackSummary,
    force: bool,
    set_candidates: WriteSignal<Vec<LyricsCandidate>>,
    set_lines: WriteSignal<Vec<LyricLine>>,
    set_text: WriteSignal<String>,
    set_loaded: WriteSignal<bool>,
    set_status: WriteSignal<String>,
) {
    if storage_get(&storage_key("lyrics_disabled", &track)).as_deref() == Some("true") && !force {
        set_text.set(String::from("Disabled"));
        return;
    }
    if let Some(saved) = storage_get(&storage_key("lyrics", &track)).filter(|_| !force) {
        apply_lyrics_text(&saved, false, Some(&track), set_lines, set_text, set_loaded);
        return;
    }
    set_text.set(String::from("Loading..."));
    spawn_result! {
        api_get::<Vec<LyricsCandidate>>(&format!("/api/lyrics/search?track_id={}", track.id)),
        Ok(candidates) => {
            set_candidates.set(candidates.clone());
            if let Some(first) = candidates.first() {
                apply_lyrics_text(
                    &first.lyrics,
                    true,
                    Some(&track),
                    set_lines,
                    set_text,
                    set_loaded,
                );
            } else {
                set_lines.set(Vec::new());
                set_loaded.set(false);
                set_text.set(String::from("Lyrics not found"));
            }
        },
        Err(err) => {
            set_lines.set(Vec::new());
            set_loaded.set(false);
            set_text.set(String::from("Lyrics not found"));
            set_status.set(err);
        },
    };
}

pub(crate) fn apply_lyrics_text(
    lyrics: &str,
    save: bool,
    track: Option<&TrackSummary>,
    set_lines: WriteSignal<Vec<LyricLine>>,
    set_text: WriteSignal<String>,
    set_loaded: WriteSignal<bool>,
) {
    let parsed = parse_lyrics(lyrics);
    if save {
        if let Some(track) = track {
            storage_set(&storage_key("lyrics", track), lyrics);
            storage_set(&storage_key("lyrics_disabled", track), "false");
        }
    }
    if parsed.lines.is_empty() {
        set_lines.set(Vec::new());
        set_loaded.set(false);
        set_text.set(if parsed.plain.is_empty() {
            String::from("Lyrics not found")
        } else {
            parsed.plain.join("\n")
        });
    } else {
        set_text.set(String::new());
        set_lines.set(parsed.lines);
        set_loaded.set(true);
    }
}

fn parse_lyrics(lyrics: &str) -> ParsedLyrics {
    let mut timed = BTreeMap::<i64, Vec<String>>::new();
    let mut plain = Vec::<String>::new();
    for raw_line in lyrics.lines() {
        let mut stamps = Vec::<i64>::new();
        let mut pos = 0_usize;
        let mut last_stamp_end = 0_usize;
        while let Some(start_rel) = raw_line[pos..].find('[') {
            let start = pos + start_rel;
            let Some(end_rel) = raw_line[start + 1..].find(']') else {
                break;
            };
            let end = start + 1 + end_rel;
            if let Some(time_ms) = parse_timestamp(&raw_line[start + 1..end]) {
                stamps.push(time_ms);
                last_stamp_end = end + 1;
            }
            pos = end + 1;
        }
        if stamps.is_empty() {
            let text = raw_line.trim();
            if !text.is_empty() {
                plain.push(text.to_string());
            }
            continue;
        }
        let text = raw_line[last_stamp_end..].trim();
        if text.is_empty() {
            continue;
        }
        for stamp in stamps {
            timed.entry(stamp).or_default().push(text.to_string());
        }
    }
    ParsedLyrics {
        lines: timed
            .into_iter()
            .map(|(time_ms, texts)| LyricLine {
                time_ms,
                text: texts.first().cloned().unwrap_or_default(),
                translations: texts.into_iter().skip(1).collect(),
            })
            .collect(),
        plain,
    }
}

fn parse_timestamp(stamp: &str) -> Option<i64> {
    let (minutes, rest) = stamp.split_once(':')?;
    let minutes = minutes.parse::<i64>().ok()?;
    let (seconds, fraction) = rest
        .find(['.', ':'])
        .map(|index| (&rest[..index], &rest[index + 1..]))
        .unwrap_or((rest, ""));
    let seconds = seconds.parse::<i64>().ok()?;
    if seconds > 59 {
        return None;
    }
    let millis = if fraction.is_empty() {
        0
    } else {
        let padded = format!("{fraction:0<3}");
        padded[..3.min(padded.len())].parse::<i64>().ok()?
    };
    Some((minutes * 60 + seconds) * 1000 + millis)
}

pub(crate) fn active_lyric_index(lines: &[LyricLine], now_ms: i64) -> i64 {
    if lines.is_empty() || now_ms < lines[0].time_ms {
        return -1;
    }
    let mut low = 0_usize;
    let mut high = lines.len() - 1;
    while low <= high {
        let mid = (low + high) / 2;
        if lines[mid].time_ms <= now_ms {
            low = mid + 1;
        } else if mid == 0 {
            break;
        } else {
            high = mid - 1;
        }
    }
    high as i64
}

pub(crate) fn storage_key(kind: &str, track: &TrackSummary) -> String {
    let id = if track.uuid.is_empty() {
        track.id.to_string()
    } else {
        track.uuid.clone()
    };
    format!("{kind}_{id}")
}

fn storage_get(key: &str) -> Option<String> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(key).ok().flatten())
}

pub(crate) fn storage_set(key: &str, value: &str) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item(key, value);
    }
}
