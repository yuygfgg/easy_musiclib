use crate::api::api_get;
use easy_musiclib_macros::spawn_async;
use easy_musiclib_shared::{AppSettings, BrowserPlaybackFormat, TrackSummary};
use gloo_net::http::Request;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use wasm_bindgen::JsCast;

const HLS_PREFETCH_TRACK_LIMIT: usize = 16;
const HLS_PREFETCH_SEGMENT_COUNT: usize = 3;

thread_local! {
    static PREFETCH_EPOCH: Cell<u64> = const { Cell::new(0) };
    static PREFETCHED_TRACKS: RefCell<HashSet<i64>> = RefCell::new(HashSet::new());
}

pub(crate) fn hls_url(track_id: i64) -> String {
    hls_file_url(track_id, "playlist.m3u8")
}

pub(crate) fn audio_supports_flac_hls(
    audio: &web_sys::HtmlAudioElement,
    format: BrowserPlaybackFormat,
) -> bool {
    if format != BrowserPlaybackFormat::Flac48k {
        return false;
    }
    audio
        .can_play_type("application/vnd.apple.mpegurl; codecs=\"fLaC\"")
        .eq("probably")
        || audio
            .can_play_type("audio/mpegurl; codecs=\"fLaC\"")
            .eq("probably")
}

pub(crate) fn spawn_hls_page_prefetch(tracks: Vec<TrackSummary>) {
    if tracks.is_empty() {
        return;
    }

    let epoch = next_prefetch_epoch();
    spawn_async! {
        let Ok(settings) = api_get::<AppSettings>("/api/settings").await else {
            return;
        };
        if !is_active_prefetch_epoch(epoch) {
            return;
        }
        if !browser_supports_flac_hls(settings.browser_playback_format) {
            return;
        }
        prefetch_hls_track_ids(hls_page_prefetch_track_ids(&tracks), epoch);
    };
}

pub(crate) fn prefetch_hls_playlist_tracks(
    tracks: &[TrackSummary],
    playlist_index: i64,
    current_track_id: Option<i64>,
) {
    let epoch = next_prefetch_epoch();
    prefetch_hls_track_ids(
        hls_playlist_prefetch_track_ids(tracks, playlist_index, current_track_id),
        epoch,
    );
}

fn browser_supports_flac_hls(format: BrowserPlaybackFormat) -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Some(document) = window.document() else {
        return false;
    };
    let Ok(element) = document.create_element("audio") else {
        return false;
    };
    let Ok(audio) = element.dyn_into::<web_sys::HtmlAudioElement>() else {
        return false;
    };
    audio_supports_flac_hls(&audio, format)
}

fn next_prefetch_epoch() -> u64 {
    PREFETCH_EPOCH.with(|cell| {
        let epoch = cell.get().wrapping_add(1);
        cell.set(epoch);
        epoch
    })
}

fn prefetch_hls_track_ids(track_ids: Vec<i64>, epoch: u64) {
    if track_ids.is_empty() {
        return;
    }

    let pending = PREFETCHED_TRACKS.with(|prefetched| {
        let prefetched = prefetched.borrow();
        track_ids
            .into_iter()
            .filter(|track_id| !prefetched.contains(track_id))
            .collect::<Vec<_>>()
    });
    if pending.is_empty() {
        return;
    }
    spawn_async! {
        for track_id in pending {
            if !is_active_prefetch_epoch(epoch) {
                break;
            }
            let should_prefetch = PREFETCHED_TRACKS.with(|prefetched| {
                let mut prefetched = prefetched.borrow_mut();
                prefetched.insert(track_id)
            });
            if !should_prefetch {
                continue;
            }
            prefetch_hls_start(track_id, epoch).await;
        }
    };
}

fn is_active_prefetch_epoch(epoch: u64) -> bool {
    PREFETCH_EPOCH.with(|cell| cell.get() == epoch)
}

fn hls_file_url(track_id: i64, file: &str) -> String {
    format!("/api/tracks/{track_id}/hls/{file}")
}

async fn prefetch_hls_start(track_id: i64, epoch: u64) {
    let response = match Request::get(&hls_url(track_id)).send().await {
        Ok(response) => response,
        Err(_) => return,
    };
    if !is_active_prefetch_epoch(epoch) {
        return;
    }
    let playlist = response.text().await.unwrap_or_default();

    let mut urls = Vec::with_capacity(HLS_PREFETCH_SEGMENT_COUNT + 1);
    urls.push(hls_file_url(track_id, "init.mp4"));
    urls.extend(hls_playlist_segment_urls(
        track_id,
        &playlist,
        HLS_PREFETCH_SEGMENT_COUNT,
    ));

    for url in urls {
        if !is_active_prefetch_epoch(epoch) {
            break;
        }
        let _ = Request::get(&url).send().await;
    }
}

fn hls_playlist_segment_urls(track_id: i64, playlist: &str, limit: usize) -> Vec<String> {
    playlist
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && line.ends_with(".m4s"))
        .take(limit)
        .map(|line| {
            if line.contains("://") || line.starts_with('/') {
                line.to_owned()
            } else {
                hls_file_url(track_id, line)
            }
        })
        .collect()
}

fn hls_page_prefetch_track_ids(tracks: &[TrackSummary]) -> Vec<i64> {
    let mut seen = HashSet::new();
    tracks
        .iter()
        .filter(|track| track.playable && seen.insert(track.id))
        .map(|track| track.id)
        .take(HLS_PREFETCH_TRACK_LIMIT)
        .collect()
}

fn hls_playlist_prefetch_track_ids(
    tracks: &[TrackSummary],
    playlist_index: i64,
    current_track_id: Option<i64>,
) -> Vec<i64> {
    if tracks.is_empty() {
        return Vec::new();
    }

    let active_index = usize::try_from(playlist_index)
        .ok()
        .filter(|index| *index < tracks.len());
    let start = active_index
        .map(|index| (index + 1) % tracks.len())
        .unwrap_or(0);
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    for offset in 0..tracks.len() {
        let index = (start + offset) % tracks.len();
        let track = &tracks[index];
        if Some(index) == active_index || Some(track.id) == current_track_id {
            continue;
        }
        if track.playable && seen.insert(track.id) {
            ids.push(track.id);
            if ids.len() >= HLS_PREFETCH_TRACK_LIMIT {
                break;
            }
        }
    }
    ids
}
