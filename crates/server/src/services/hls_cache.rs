use crate::application::playback::{
    HLS_INIT_FILE, HLS_PLAYLIST_FILE, HlsRenderRequest, PlaybackMedia,
};
use crate::domain::{PlaybackSource, TrackId};
use crate::{ApiResult, AppError};
use anyhow::Context;
use easy_musiclib_shared::HlsCacheClearResponse;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;
use tokio::time::{Duration, Instant, sleep};

pub async fn clear_hls_cache() -> ApiResult<HlsCacheClearResponse> {
    let root = hls_cache_root();
    let summary = tokio::task::spawn_blocking(move || {
        let active = hls_generators()
            .lock()
            .map_err(|_| anyhow::anyhow!("HLS generator lock poisoned"))?;
        clear_hls_cache_root(&root, &active)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))??;
    Ok(summary)
}

pub async fn hls_cache_dir(track_id: TrackId, source: &PlaybackSource) -> ApiResult<PathBuf> {
    let metadata = tokio::fs::metadata(&source.path).await?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let mut hasher = Sha256::new();
    hasher.update(b"flac-48k-fmp4-hls-v1");
    hasher.update(track_id.raw().to_le_bytes());
    hasher.update(source.path.as_bytes());
    hasher.update(source.renderer.as_bytes());
    hasher.update(source.codec.as_bytes());
    hasher.update(source.start_ms.unwrap_or(0).to_le_bytes());
    hasher.update(source.end_ms.unwrap_or(-1).to_le_bytes());
    hasher.update(metadata.len().to_le_bytes());
    hasher.update(modified.to_le_bytes());
    Ok(hls_cache_root().join(hex::encode(hasher.finalize())))
}

pub fn hls_file_path(cache_dir: &Path, file: &str) -> ApiResult<PathBuf> {
    if file.contains('/') || file.contains('\\') || file.contains("..") {
        return Err(AppError::bad_request("invalid HLS file name"));
    }
    if file == HLS_PLAYLIST_FILE || file == HLS_INIT_FILE || is_hls_segment_file(file) {
        return Ok(cache_dir.join(file));
    }
    Err(AppError::not_found("HLS file not found"))
}

pub fn is_hls_segment_file(file: &str) -> bool {
    let Some(index) = file
        .strip_prefix("segment_")
        .and_then(|rest| rest.strip_suffix(".m4s"))
    else {
        return false;
    };
    index.len() == 5 && index.bytes().all(|byte| byte.is_ascii_digit())
}

pub fn hls_file_timeout(file: &str) -> Duration {
    if file == HLS_PLAYLIST_FILE || file == HLS_INIT_FILE {
        Duration::from_secs(15)
    } else {
        Duration::from_secs(30)
    }
}

pub fn hls_playlist_for_playback(playlist: &str) -> String {
    if playlist.contains("#EXT-X-START:") {
        return playlist.to_owned();
    }

    let insert_after = playlist
        .lines()
        .position(|line| line.starts_with("#EXT-X-VERSION"))
        .or_else(|| playlist.lines().position(|line| line == "#EXTM3U"));
    let Some(insert_after) = insert_after else {
        return playlist.to_owned();
    };

    let mut out = String::with_capacity(playlist.len() + 48);
    for (index, line) in playlist.lines().enumerate() {
        out.push_str(line);
        out.push('\n');
        if index == insert_after {
            out.push_str("#EXT-X-START:TIME-OFFSET=0.000,PRECISE=YES\n");
        }
    }
    out
}

pub async fn ensure_hls_generation<M>(
    playback_media: M,
    source: &PlaybackSource,
    cache_dir: &Path,
) -> ApiResult<()>
where
    M: PlaybackMedia + Clone + Send + Sync + 'static,
{
    if tokio::fs::metadata(hls_complete_path(cache_dir))
        .await
        .is_ok()
    {
        return Ok(());
    }

    let cache_dir = cache_dir.to_path_buf();
    let should_start = {
        let mut active = hls_generators()
            .lock()
            .map_err(|_| AppError::internal("HLS generator lock poisoned"))?;
        active.insert(cache_dir.clone())
    };
    if !should_start {
        return Ok(());
    }

    let input_path = PathBuf::from(source.path.clone());
    let start_ms = source.start_ms.unwrap_or(0).max(0);
    let end_ms = source.end_ms;
    tokio::spawn(async move {
        let result = async {
            if cache_dir.exists() {
                tokio::fs::remove_dir_all(&cache_dir)
                    .await
                    .with_context(|| format!("removing stale HLS cache {}", cache_dir.display()))?;
            }
            tokio::fs::create_dir_all(&cache_dir)
                .await
                .with_context(|| format!("creating HLS cache {}", cache_dir.display()))?;
            playback_media
                .render_hls(HlsRenderRequest {
                    path: input_path.clone(),
                    output_dir: cache_dir.clone(),
                    start_ms,
                    end_ms,
                })
                .await?;
            tokio::fs::write(hls_complete_path(&cache_dir), b"ok")
                .await
                .with_context(|| format!("writing HLS complete marker {}", cache_dir.display()))?;
            Ok::<_, anyhow::Error>(())
        }
        .await;
        if let Err(err) = result {
            tracing::error!(
                path = %input_path.display(),
                cache_dir = %cache_dir.display(),
                error = %err,
                "failed to generate FLAC HLS"
            );
        }
        if let Ok(mut active) = hls_generators().lock() {
            active.remove(&cache_dir);
        }
    });

    Ok(())
}

pub async fn wait_for_hls_file(path: &Path, timeout: Duration) -> ApiResult<()> {
    let deadline = Instant::now() + timeout;
    loop {
        if tokio::fs::metadata(path).await.is_ok() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(AppError::not_found("HLS file is not ready"));
        }
        sleep(Duration::from_millis(50)).await;
    }
}

fn hls_cache_root() -> PathBuf {
    std::env::temp_dir().join("easy_musiclib_hls")
}

fn hls_complete_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join(".complete")
}

fn hls_generators() -> &'static Mutex<HashSet<PathBuf>> {
    static ACTIVE: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    ACTIVE.get_or_init(|| Mutex::new(HashSet::new()))
}

fn clear_hls_cache_root(
    root: &Path,
    active_dirs: &HashSet<PathBuf>,
) -> anyhow::Result<HlsCacheClearResponse> {
    let mut summary = HlsCacheClearResponse {
        cache_dir: root.to_string_lossy().into_owned(),
        removed_files: 0,
        removed_dirs: 0,
        removed_bytes: 0,
        skipped_active_generators: 0,
    };

    match std::fs::symlink_metadata(root) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => {
            remove_hls_cache_path(root, &mut summary)?;
            return Ok(summary);
        }
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(summary),
        Err(err) => {
            return Err(err).with_context(|| format!("reading HLS cache root {}", root.display()));
        }
    }

    if active_dirs.is_empty() {
        remove_hls_cache_path(root, &mut summary)?;
        return Ok(summary);
    }

    for entry in std::fs::read_dir(root)
        .with_context(|| format!("reading HLS cache root {}", root.display()))?
    {
        let path = entry
            .with_context(|| format!("reading HLS cache root {}", root.display()))?
            .path();
        if active_dirs.contains(&path) {
            summary.skipped_active_generators += 1;
            continue;
        }
        remove_hls_cache_path(&path, &mut summary)?;
    }

    match std::fs::remove_dir(root) {
        Ok(()) => summary.removed_dirs += 1,
        Err(err)
            if matches!(
                err.kind(),
                ErrorKind::NotFound | ErrorKind::DirectoryNotEmpty
            ) => {}
        Err(err) => {
            return Err(err).with_context(|| format!("removing HLS cache root {}", root.display()));
        }
    }

    Ok(summary)
}

fn remove_hls_cache_path(path: &Path, summary: &mut HlsCacheClearResponse) -> anyhow::Result<()> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(err).with_context(|| format!("reading HLS cache path {}", path.display()));
        }
    };

    if metadata.is_dir() {
        for entry in
            std::fs::read_dir(path).with_context(|| format!("reading {}", path.display()))?
        {
            let path = entry
                .with_context(|| format!("reading {}", path.display()))?
                .path();
            remove_hls_cache_path(&path, summary)?;
        }
        match std::fs::remove_dir(path) {
            Ok(()) => summary.removed_dirs += 1,
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("removing HLS cache directory {}", path.display()));
            }
        }
    } else {
        let bytes = metadata.len();
        match std::fs::remove_file(path) {
            Ok(()) => {
                summary.removed_files += 1;
                summary.removed_bytes = summary.removed_bytes.saturating_add(bytes);
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("removing HLS cache file {}", path.display()));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hls_playlist_for_playback_prefers_zero_start_for_event_playlist() {
        let playlist = "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-PLAYLIST-TYPE:EVENT\n#EXT-X-MAP:URI=\"init.mp4\"\n#EXTINF:2.000000,\nsegment_00000.m4s\n";

        let rewritten = hls_playlist_for_playback(playlist);

        assert!(rewritten.contains(
            "#EXT-X-VERSION:7\n#EXT-X-START:TIME-OFFSET=0.000,PRECISE=YES\n#EXT-X-PLAYLIST-TYPE:EVENT"
        ));
    }

    #[test]
    fn hls_playlist_for_playback_keeps_existing_start_tag() {
        let playlist = "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-START:TIME-OFFSET=0.000,PRECISE=YES\n#EXT-X-PLAYLIST-TYPE:VOD\n";

        assert_eq!(hls_playlist_for_playback(playlist), playlist);
    }
}
