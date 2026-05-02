use axum::http::HeaderMap;
use axum::http::header::RANGE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestedRange {
    None,
    Valid(u64, u64),
    Invalid,
}

pub fn requested_range(headers: &HeaderMap, len: u64) -> RequestedRange {
    let Some(range) = headers.get(RANGE) else {
        return RequestedRange::None;
    };
    let Ok(range) = range.to_str() else {
        return RequestedRange::Invalid;
    };
    parse_range(range, len)
        .map(|(start, end)| RequestedRange::Valid(start, end))
        .unwrap_or(RequestedRange::Invalid)
}

fn parse_range(range: &str, len: u64) -> Option<(u64, u64)> {
    if len == 0 {
        return None;
    }
    let range = range.strip_prefix("bytes=")?;
    let (start, end) = range.split_once('-')?;
    if start.is_empty() {
        let suffix = end.parse::<u64>().ok()?;
        if suffix == 0 {
            return None;
        }
        let start = len.saturating_sub(suffix);
        return Some((start, len.saturating_sub(1)));
    }
    let start = start.parse::<u64>().ok()?;
    let end = if end.is_empty() {
        len.saturating_sub(1)
    } else {
        end.parse::<u64>().ok()?.min(len.saturating_sub(1))
    };
    (start <= end && end < len).then_some((start, end))
}
