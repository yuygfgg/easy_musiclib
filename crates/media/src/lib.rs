pub mod artists;
pub mod cue;
mod flac_split;
pub mod normalize;
pub mod render;
pub mod tags;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredFile {
    pub path: PathBuf,
    pub path_hash: String,
    pub size: i64,
    pub mtime_ns: i64,
    pub format: String,
}

pub fn supported_extension(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match ext.as_str() {
        "flac" => Some("flac"),
        "mp3" => Some("mp3"),
        "wav" => Some("wav"),
        "cue" => Some("cue"),
        _ => None,
    }
}

pub fn is_audio_extension(path: &Path) -> bool {
    matches!(supported_extension(path), Some("flac" | "mp3" | "wav"))
}

pub fn discover_files(root_paths: &[String]) -> Result<Vec<DiscoveredFile>> {
    let mut out = Vec::new();
    for root in root_paths {
        for entry in WalkDir::new(root).follow_links(false).into_iter() {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            if !entry.file_type().is_file() {
                continue;
            }
            let Some(format) = supported_extension(entry.path()) else {
                continue;
            };
            let meta = match fs::metadata(entry.path()) {
                Ok(meta) => meta,
                Err(_) => continue,
            };
            out.push(DiscoveredFile {
                path: entry.path().to_path_buf(),
                path_hash: path_hash(entry.path()),
                size: meta.len().try_into().unwrap_or(i64::MAX),
                mtime_ns: mtime_ns(&meta),
                format: format.to_string(),
            });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

pub fn path_hash(path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(unix)]
fn mtime_ns(meta: &fs::Metadata) -> i64 {
    use std::os::unix::fs::MetadataExt;
    meta.mtime()
        .saturating_mul(1_000_000_000)
        .saturating_add(meta.mtime_nsec())
}

#[cfg(not(unix))]
fn mtime_ns(meta: &fs::Metadata) -> i64 {
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
