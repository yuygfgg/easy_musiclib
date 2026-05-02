use crate::{cue, formats, mtime_ns, path_hash};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidedFile {
    pub path: PathBuf,
    pub path_hash: String,
    pub size: i64,
    pub mtime_ns: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredAudioFile {
    pub path: PathBuf,
    pub path_hash: String,
    pub size: i64,
    pub mtime_ns: i64,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredCueFile {
    pub path: PathBuf,
    pub path_hash: String,
    pub size: i64,
    pub mtime_ns: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscoveredLibraryFiles {
    pub audio: Vec<DiscoveredAudioFile>,
    pub cues: Vec<DiscoveredCueFile>,
}

impl DiscoveredLibraryFiles {
    pub fn len(&self) -> usize {
        self.audio.len() + self.cues.len()
    }

    pub fn is_empty(&self) -> bool {
        self.audio.is_empty() && self.cues.is_empty()
    }
}

pub trait FileProvider {
    fn discover_files(&self, root_paths: &[String]) -> Result<Vec<ProvidedFile>>;
}

pub trait CueProvider {
    fn discover_cues(&self, files: &[ProvidedFile]) -> Vec<DiscoveredCueFile>;
}

pub struct FilesystemFileProvider;

impl FileProvider for FilesystemFileProvider {
    fn discover_files(&self, root_paths: &[String]) -> Result<Vec<ProvidedFile>> {
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
                let meta = match fs::metadata(entry.path()) {
                    Ok(meta) => meta,
                    Err(_) => continue,
                };
                out.push(provided_file(entry.path(), &meta));
            }
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }
}

pub struct ExtensionCueProvider;

impl CueProvider for ExtensionCueProvider {
    fn discover_cues(&self, files: &[ProvidedFile]) -> Vec<DiscoveredCueFile> {
        let mut out = files
            .iter()
            .filter(|file| cue::is_cue_path(&file.path))
            .map(|file| DiscoveredCueFile {
                path: file.path.clone(),
                path_hash: file.path_hash.clone(),
                size: file.size,
                mtime_ns: file.mtime_ns,
            })
            .collect::<Vec<_>>();
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out
    }
}

pub fn discover_library_files(root_paths: &[String]) -> Result<DiscoveredLibraryFiles> {
    discover_library_files_with(root_paths, &FilesystemFileProvider, &ExtensionCueProvider)
}

pub fn discover_library_files_with(
    root_paths: &[String],
    file_provider: &dyn FileProvider,
    cue_provider: &dyn CueProvider,
) -> Result<DiscoveredLibraryFiles> {
    let files = file_provider.discover_files(root_paths)?;
    let mut audio = discover_audio_files(&files);
    let cues = cue_provider.discover_cues(&files);
    audio.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(DiscoveredLibraryFiles { audio, cues })
}

fn discover_audio_files(files: &[ProvidedFile]) -> Vec<DiscoveredAudioFile> {
    files
        .iter()
        .filter_map(|file| {
            let format = formats::format_by_extension(&file.path)?;
            Some(DiscoveredAudioFile {
                path: file.path.clone(),
                path_hash: file.path_hash.clone(),
                size: file.size,
                mtime_ns: file.mtime_ns,
                format: format.id().to_string(),
            })
        })
        .collect()
}

fn provided_file(path: &Path, meta: &fs::Metadata) -> ProvidedFile {
    ProvidedFile {
        path: path.to_path_buf(),
        path_hash: path_hash(path),
        size: meta.len().try_into().unwrap_or(i64::MAX),
        mtime_ns: mtime_ns(meta),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFileProvider {
        files: Vec<ProvidedFile>,
    }

    impl FileProvider for FakeFileProvider {
        fn discover_files(&self, _root_paths: &[String]) -> Result<Vec<ProvidedFile>> {
            Ok(self.files.clone())
        }
    }

    #[test]
    fn splits_audio_and_cue_files_above_raw_file_discovery() {
        let files = vec![
            test_file("album.flac"),
            test_file("album.cue"),
            test_file("notes.txt"),
        ];
        let discovered =
            discover_library_files_with(&[], &FakeFileProvider { files }, &ExtensionCueProvider)
                .unwrap();

        assert_eq!(discovered.audio.len(), 1);
        assert_eq!(discovered.audio[0].format, "flac");
        assert_eq!(discovered.cues.len(), 1);
        assert_eq!(discovered.cues[0].path, PathBuf::from("album.cue"));
    }

    fn test_file(path: &str) -> ProvidedFile {
        ProvidedFile {
            path: PathBuf::from(path),
            path_hash: path.to_string(),
            size: 0,
            mtime_ns: 0,
        }
    }
}
