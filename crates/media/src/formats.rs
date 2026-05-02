use crate::{flac, mp3, wav};
use anyhow::{Context, Result};
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub trait AudioFormat: Sync + Send {
    fn id(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];
    fn mime(&self) -> Option<&'static str>;

    fn matches_extension(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                self.extensions()
                    .iter()
                    .any(|supported| ext.eq_ignore_ascii_case(supported))
            })
            .unwrap_or(false)
    }

    fn sniff(&self, path: &Path) -> Result<bool> {
        Ok(self.matches_extension(path))
    }
}

pub fn audio_formats() -> &'static [&'static dyn AudioFormat] {
    &AUDIO_FORMATS
}

pub fn format_by_id(id: &str) -> Option<&'static dyn AudioFormat> {
    audio_formats()
        .iter()
        .copied()
        .find(|format| format.id().eq_ignore_ascii_case(id))
}

pub fn format_by_extension(path: &Path) -> Option<&'static dyn AudioFormat> {
    audio_formats()
        .iter()
        .copied()
        .find(|format| format.matches_extension(path))
}

pub fn detect_format(path: &Path) -> Result<Option<&'static dyn AudioFormat>> {
    if let Some(format) = format_by_extension(path) {
        return Ok(Some(format));
    }
    sniff_format(path)
}

pub fn sniff_format(path: &Path) -> Result<Option<&'static dyn AudioFormat>> {
    for format in audio_formats() {
        if format.sniff(path)? {
            return Ok(Some(*format));
        }
    }
    Ok(None)
}

pub(crate) fn read_prefix(path: &Path, buf: &mut [u8]) -> Result<usize> {
    let mut file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    file.read(buf)
        .with_context(|| format!("reading {}", path.display()))
}

pub(crate) fn best_by_priority<T: ?Sized>(
    items: &'static [&'static T],
    priority: impl Fn(&'static T) -> Option<i32>,
) -> Option<&'static T> {
    items
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(idx, item)| priority(item).map(|score| (score, std::cmp::Reverse(idx), item)))
        .max_by_key(|(score, reverse_idx, _)| (*score, *reverse_idx))
        .map(|(_, _, item)| item)
}

static AUDIO_FORMATS: [&'static dyn AudioFormat; 3] = [&flac::FORMAT, &mp3::FORMAT, &wav::FORMAT];

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn maps_existing_extensions_to_formats() {
        assert_eq!(
            format_by_extension(Path::new("a.flac")).map(|format| format.id()),
            Some("flac")
        );
        assert_eq!(
            format_by_extension(Path::new("a.MP3")).map(|format| format.id()),
            Some("mp3")
        );
        assert_eq!(
            format_by_extension(Path::new("a.wav")).map(|format| format.id()),
            Some("wav")
        );
        assert!(format_by_extension(Path::new("a.cue")).is_none());
        assert!(format_by_extension(Path::new("a.ogg")).is_none());
    }

    #[test]
    fn priority_ties_keep_registration_order() {
        assert_eq!(
            best_by_priority(&[&1_i32, &2_i32], |_| Some(10)).copied(),
            Some(1)
        );
    }
}
