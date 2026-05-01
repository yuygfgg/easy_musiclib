pub mod artists;
pub mod cue;
mod encoding;
mod ffmpeg_backend;
mod flac;
pub mod formats;
mod mp3;
pub mod normalize;
pub mod providers;
pub mod render;
pub mod tags;
mod wav;

use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

pub fn path_hash(path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(unix)]
pub(crate) fn mtime_ns(meta: &fs::Metadata) -> i64 {
    use std::os::unix::fs::MetadataExt;
    meta.mtime()
        .saturating_mul(1_000_000_000)
        .saturating_add(meta.mtime_nsec())
}

#[cfg(not(unix))]
pub(crate) fn mtime_ns(meta: &fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos().try_into().unwrap_or(i64::MAX))
        .unwrap_or(0)
}

pub fn extract_year(input: Option<&str>) -> Option<i64> {
    let input = input?;
    let mut digits = String::new();
    for ch in input.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            if digits.len() == 4 {
                return digits.parse::<i64>().ok();
            }
        } else {
            digits.clear();
        }
    }
    None
}

pub fn clean_file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Unknown Title".to_string())
}
