use crate::artists::parse_artists;
use crate::{clean_file_stem, extract_year};
use anyhow::{Context, Result};
use lofty::config::ParseOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::prelude::Accessor;
use lofty::probe::Probe;
use lofty::tag::{ItemKey, ItemValue, Tag};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTags {
    pub title: String,
    pub album: String,
    pub artists: Vec<String>,
    pub album_artists: Vec<String>,
    pub raw_artists: Vec<String>,
    pub raw_album_artists: Vec<String>,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,
    pub date: Option<String>,
    pub year: Option<i64>,
    pub event: Option<String>,
    pub duration_ms: Option<i64>,
    pub sample_rate: Option<i64>,
    pub channels: Option<i64>,
    pub embedded_picture: Option<EmbeddedPictureInfo>,
    pub sidecar_artwork: Option<PathBuf>,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedPictureInfo {
    pub index: i64,
    pub mime: Option<String>,
}

pub fn read_audio_tags(path: &Path, split_exceptions: &[String]) -> Result<AudioTags> {
    let sidecar_artwork = find_sidecar_artwork(path);
    let read_embedded_artwork = sidecar_artwork.is_none();
    let tagged = Probe::open(path)
        .with_context(|| format!("opening audio metadata {}", path.display()))?
        .options(ParseOptions::new().read_cover_art(read_embedded_artwork))
        .read()
        .with_context(|| format!("reading audio metadata {}", path.display()))?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
    let properties = tagged.properties();
    let title = tag
        .and_then(|t| t.title().map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| clean_file_stem(path));
    let album = tag
        .and_then(|t| t.album().map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Unknown Album".to_string());

    let raw_artists = tag
        .map(read_track_artists)
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| vec!["Unknown Artist".to_string()]);
    let raw_album_artists = tag
        .map(read_album_artists)
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| raw_artists.clone());
    let artists = parse_artists(&raw_artists, split_exceptions);
    let album_artists = parse_artists(&raw_album_artists, split_exceptions);

    let track_number = tag.and_then(read_track_number);
    let disc_number = tag.and_then(read_disc_number);
    let date = tag.and_then(read_date);
    let year = tag.and_then(|t| {
        t.year()
            .map(i64::from)
            .or_else(|| extract_year(date.as_deref()))
    });
    let event = tag
        .and_then(read_event)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("Unknown Event"));
    let embedded_picture = read_embedded_artwork
        .then(|| {
            tag.and_then(|t| {
                t.pictures().first().map(|picture| EmbeddedPictureInfo {
                    index: 0,
                    mime: picture.mime_type().map(|m| m.as_str().to_string()),
                })
            })
        })
        .flatten();

    Ok(AudioTags {
        title,
        album,
        artists,
        album_artists,
        raw_artists,
        raw_album_artists,
        track_number,
        disc_number,
        date,
        year,
        event,
        duration_ms: Some(
            properties
                .duration()
                .as_millis()
                .try_into()
                .unwrap_or(i64::MAX),
        ),
        sample_rate: properties.sample_rate().map(i64::from),
        channels: properties.channels().map(i64::from),
        embedded_picture,
        sidecar_artwork,
        format: format!("{:?}", tagged.file_type()).to_ascii_lowercase(),
    })
}

pub fn read_embedded_picture(path: &Path, index: i64) -> Result<(Vec<u8>, Option<String>)> {
    let tagged = lofty::read_from_path(path)
        .with_context(|| format!("reading embedded picture {}", path.display()))?;
    let tag = tagged
        .primary_tag()
        .or_else(|| tagged.first_tag())
        .context("file has no readable tag")?;
    let picture = tag
        .pictures()
        .get(index.max(0) as usize)
        .context("embedded picture index not found")?;
    Ok((
        picture.data().to_vec(),
        picture.mime_type().map(|m| m.as_str().to_string()),
    ))
}

fn read_track_artists(tag: &Tag) -> Vec<String> {
    let mut out: Vec<String> = tag
        .get_strings(&ItemKey::TrackArtists)
        .chain(tag.get_strings(&ItemKey::TrackArtist))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    if out.is_empty() {
        if let Some(artist) = tag.artist() {
            out.push(artist.trim().to_string());
        }
    }
    out
}

fn read_album_artists(tag: &Tag) -> Vec<String> {
    tag.get_strings(&ItemKey::AlbumArtist)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn read_track_number(tag: &Tag) -> Option<i64> {
    tag.get_string(&ItemKey::TrackNumber)
        .and_then(parse_leading_number)
        .or_else(|| tag.track().map(i64::from))
}

fn read_disc_number(tag: &Tag) -> Option<i64> {
    tag.get_string(&ItemKey::DiscNumber)
        .and_then(parse_leading_number)
        .or_else(|| tag.disk().map(i64::from))
}

fn read_date(tag: &Tag) -> Option<String> {
    for key in [
        ItemKey::RecordingDate,
        ItemKey::ReleaseDate,
        ItemKey::OriginalReleaseDate,
        ItemKey::Year,
    ] {
        if let Some(value) = tag.get_string(&key) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn read_event(tag: &Tag) -> Option<String> {
    for item in tag.items() {
        let key_matches = match item.key() {
            ItemKey::Unknown(key) => {
                key.eq_ignore_ascii_case("event")
                    || key.eq_ignore_ascii_case("EVENT")
                    || key.eq_ignore_ascii_case("TXXX:EVENT")
            }
            ItemKey::ContentGroup | ItemKey::Work => true,
            _ => false,
        };
        if key_matches {
            if let ItemValue::Text(text) = item.value() {
                if !text.trim().is_empty() {
                    return Some(text.clone());
                }
            }
        }
    }
    None
}

fn parse_leading_number(s: &str) -> Option<i64> {
    s.trim().split('/').next().and_then(|head| {
        head.chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<i64>()
            .ok()
    })
}

fn find_sidecar_artwork(path: &Path) -> Option<PathBuf> {
    let dir = path.parent()?;
    let names = ["cover", "folder", "front"];
    let exts = ["jpg", "jpeg", "png", "tif", "tiff", "webp", "gif"];
    for name in names {
        for ext in exts {
            for candidate in [
                dir.join(format!("{name}.{ext}")),
                dir.join(format!("{}.{ext}", uppercase_first(name))),
                dir.join(format!("{}.{ext}", name.to_ascii_uppercase())),
            ] {
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn uppercase_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
