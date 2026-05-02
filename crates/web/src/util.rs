use easy_musiclib_shared::{AlbumSummary, TrackSummary};
use serde::Serialize;

pub(crate) const PAGE_SIZE: i64 = 100;

pub(crate) fn nav_class(active: bool) -> &'static str {
    if active {
        "nav-button active"
    } else {
        "nav-button"
    }
}

pub(crate) fn paged_status(status: String, page: i64, total: i64) -> String {
    let pages = total_pages(total);
    if total == 0 {
        status
    } else if pages > 1 {
        format!("{status} · page {page} / {pages}")
    } else {
        status
    }
}

pub(crate) fn total_pages(total: i64) -> i64 {
    ((total.max(0) + PAGE_SIZE - 1) / PAGE_SIZE).max(1)
}

pub(crate) fn album_date(date: &Option<String>, year: Option<i64>) -> String {
    date.clone()
        .or_else(|| year.map(|value| value.to_string()))
        .unwrap_or_default()
}

pub(crate) fn album_counts(album: &AlbumSummary) -> String {
    if album.disc_count > 1 {
        format!("{} discs · {} tracks", album.disc_count, album.song_count)
    } else {
        format!("{} tracks", album.song_count)
    }
}

pub(crate) fn format_time(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return String::from("0:00");
    }
    let total = seconds.floor() as i64;
    format!("{}:{:02}", total / 60, total % 60)
}

pub(crate) fn progress_value(current: f64, duration: f64) -> i64 {
    if duration.is_finite() && duration > 0.0 && current.is_finite() {
        ((current / duration) * 1000.0).round().clamp(0.0, 1000.0) as i64
    } else {
        0
    }
}
pub(crate) fn pretty_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|err| err.to_string())
}

pub(crate) fn playable_tracks(tracks: Vec<TrackSummary>) -> Vec<TrackSummary> {
    tracks
        .into_iter()
        .filter(|track| track.playable)
        .collect::<Vec<_>>()
}
