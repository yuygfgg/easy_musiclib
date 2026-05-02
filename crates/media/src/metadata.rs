use crate::formats::{format_by_extension, format_by_id};
use crate::tags::{AudioTags, read_audio_tags, read_embedded_picture};
use anyhow::{Context, Result, anyhow};
use std::path::Path;

pub trait MetadataReader: Sync + Send {
    fn id(&self) -> &'static str;
    fn audio_tags_priority(&self, path: &Path, format_id: Option<&str>) -> Option<i32>;
    fn embedded_picture_priority(&self, path: &Path, format_id: Option<&str>) -> Option<i32>;
    fn read_audio_tags(&self, path: &Path, split_exceptions: &[String]) -> Result<AudioTags>;
    fn read_embedded_picture(&self, path: &Path, index: i64) -> Result<(Vec<u8>, Option<String>)>;
}

pub static LOFTY_METADATA_READER: LoftyMetadataReader = LoftyMetadataReader;

pub struct LoftyMetadataReader;

impl MetadataReader for LoftyMetadataReader {
    fn id(&self) -> &'static str {
        "lofty"
    }

    fn audio_tags_priority(&self, _path: &Path, format_id: Option<&str>) -> Option<i32> {
        known_format_id(format_id).then_some(100)
    }

    fn embedded_picture_priority(&self, _path: &Path, format_id: Option<&str>) -> Option<i32> {
        known_format_id(format_id).then_some(100)
    }

    fn read_audio_tags(&self, path: &Path, split_exceptions: &[String]) -> Result<AudioTags> {
        read_audio_tags(path, split_exceptions)
    }

    fn read_embedded_picture(&self, path: &Path, index: i64) -> Result<(Vec<u8>, Option<String>)> {
        read_embedded_picture(path, index)
    }
}

pub fn metadata_readers() -> &'static [&'static dyn MetadataReader] {
    &METADATA_READERS
}

pub fn read_audio_metadata(path: &Path, split_exceptions: &[String]) -> Result<AudioTags> {
    let format_id = format_by_extension(path).map(|format| format.id());
    let reader = best_metadata_reader(|reader| reader.audio_tags_priority(path, format_id))
        .ok_or_else(|| anyhow!("no metadata reader available for {}", path.display()))?;
    reader
        .read_audio_tags(path, split_exceptions)
        .with_context(|| {
            format!(
                "metadata reader {} failed for {}",
                reader.id(),
                path.display()
            )
        })
}

pub fn read_embedded_picture_for_path(
    path: &Path,
    index: i64,
) -> Result<(Vec<u8>, Option<String>)> {
    let format_id = format_by_extension(path).map(|format| format.id());
    let reader = best_metadata_reader(|reader| reader.embedded_picture_priority(path, format_id))
        .ok_or_else(|| {
        anyhow!(
            "no embedded picture reader available for {}",
            path.display()
        )
    })?;
    reader.read_embedded_picture(path, index).with_context(|| {
        format!(
            "picture reader {} failed for {}",
            reader.id(),
            path.display()
        )
    })
}

static METADATA_READERS: [&'static dyn MetadataReader; 1] = [&LOFTY_METADATA_READER];

fn known_format_id(format_id: Option<&str>) -> bool {
    format_id.and_then(format_by_id).is_some()
}

fn best_metadata_reader(
    priority: impl Fn(&'static dyn MetadataReader) -> Option<i32>,
) -> Option<&'static dyn MetadataReader> {
    crate::formats::best_by_priority(metadata_readers(), priority)
}
