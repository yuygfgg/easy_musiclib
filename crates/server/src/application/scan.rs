use anyhow::Result;
use futures::future::BoxFuture;
use std::path::{Path, PathBuf};

pub const CUE_FORMAT_ID: &str = "cue";

#[derive(Debug, Clone, Default)]
pub struct DiscoveredLibraryFiles {
    pub audio: Vec<DiscoveredAudioFile>,
    pub cues: Vec<DiscoveredCueFile>,
}

impl DiscoveredLibraryFiles {
    pub fn len(&self) -> usize {
        self.audio.len() + self.cues.len()
    }
}

#[derive(Debug, Clone)]
pub struct DiscoveredAudioFile {
    pub path: PathBuf,
    pub path_hash: String,
    pub size: i64,
    pub mtime_ns: i64,
    pub format: String,
}

#[derive(Debug, Clone)]
pub struct DiscoveredCueFile {
    pub path: PathBuf,
    pub path_hash: String,
    pub size: i64,
    pub mtime_ns: i64,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct EmbeddedPictureInfo {
    pub index: i64,
    pub mime: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CueSheet {
    pub path: PathBuf,
    pub audio_path: PathBuf,
    pub album_title: Option<String>,
    pub performer: Option<String>,
    pub date: Option<String>,
    pub tracks: Vec<CueTrack>,
}

#[derive(Debug, Clone)]
pub struct CueTrack {
    pub no: i64,
    pub title: Option<String>,
    pub performer: Option<String>,
    pub start_frames: i64,
    pub start_ms: i64,
    pub end_ms: Option<i64>,
    pub start_sample: Option<i64>,
    pub end_sample: Option<i64>,
}

pub trait LibraryFileDiscovery: Send + Sync {
    fn discover_library_files<'a>(
        &'a self,
        roots: &'a [String],
    ) -> BoxFuture<'a, Result<DiscoveredLibraryFiles>>;
}

pub trait AudioMetadataReader: Send + Sync {
    fn read_audio_metadata<'a>(
        &'a self,
        path: &'a Path,
        split_exceptions: &'a [String],
    ) -> BoxFuture<'a, Result<AudioTags>>;
}

pub trait CueSheetReader: Send + Sync {
    fn parse_cue_file<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, Result<CueSheet>>;
}

pub trait CueRendererSelector: Send + Sync {
    fn passthrough_renderer(&self) -> &'static str;

    fn cue_renderer_id_for_format_id(&self, format_id: &str) -> &'static str;
}

pub trait ArtistNameParser: Send + Sync {
    fn parse_artists(&self, raw: &[String], split_exceptions: &[String]) -> Vec<String>;
}

pub trait ScanLibraryRepository: Send + Sync {
    fn ensure_default_split_exceptions(&self) -> BoxFuture<'_, Result<()>>;

    fn split_exceptions(&self) -> BoxFuture<'_, Result<Vec<String>>>;

    fn upsert_media_file<'a>(
        &'a self,
        path: &'a str,
        path_hash: &'a str,
        size: i64,
        mtime_ns: i64,
        format: &'a str,
    ) -> BoxFuture<'a, Result<(i64, bool)>>;

    fn media_file_has_audio_sources(&self, media_file_id: i64) -> BoxFuture<'_, Result<bool>>;

    fn set_media_file_audio_metadata(
        &self,
        media_file_id: i64,
        sample_rate: Option<i64>,
        channels: Option<i64>,
        duration_ms: Option<i64>,
    ) -> BoxFuture<'_, Result<()>>;

    fn set_media_file_scan_error<'a>(
        &'a self,
        media_file_id: i64,
        error: &'a str,
    ) -> BoxFuture<'a, Result<()>>;

    fn delete_tracks_for_media_file(&self, media_file_id: i64) -> BoxFuture<'_, Result<()>>;

    fn delete_cue_sheet_for_file(&self, cue_file_id: i64) -> BoxFuture<'_, Result<()>>;

    fn cue_sheet_exists_for_file(&self, cue_file_id: i64) -> BoxFuture<'_, Result<bool>>;

    fn insert_cue_sheet<'a>(
        &'a self,
        cue_file_id: i64,
        audio_file_id: i64,
        album_title: Option<&'a str>,
        performer: Option<&'a str>,
        date: Option<&'a str>,
    ) -> BoxFuture<'a, Result<i64>>;

    fn ensure_artist<'a>(
        &'a self,
        name: &'a str,
        artwork_id: Option<i64>,
    ) -> BoxFuture<'a, Result<i64>>;

    fn ensure_event<'a>(
        &'a self,
        name: Option<&'a str>,
        date: Option<&'a str>,
        year: Option<i64>,
    ) -> BoxFuture<'a, Result<Option<i64>>>;

    fn is_unknown_event_name(&self, name: &str) -> bool;

    fn find_or_create_album<'a>(
        &'a self,
        title: &'a str,
        album_artist_ids: &'a [i64],
        year: Option<i64>,
        date: Option<&'a str>,
        event_id: Option<i64>,
        artwork_id: Option<i64>,
    ) -> BoxFuture<'a, Result<i64>>;

    fn link_event_album(&self, event_id: i64, album_id: i64) -> BoxFuture<'_, Result<()>>;

    fn insert_track<'a>(
        &'a self,
        new_track: NewTrack<'a>,
        artist_ids: &'a [i64],
    ) -> BoxFuture<'a, Result<i64>>;

    fn insert_track_audio_source<'a>(
        &'a self,
        track_id: i64,
        src: NewTrackAudioSource<'a>,
    ) -> BoxFuture<'a, Result<()>>;

    fn ensure_artwork_source<'a>(
        &'a self,
        kind: &'a str,
        media_file_id: Option<i64>,
        sidecar_path: Option<&'a str>,
        embedded_picture_index: Option<i64>,
        mime: Option<&'a str>,
    ) -> BoxFuture<'a, Result<Option<i64>>>;

    fn refresh_track_search(&self, track_id: i64) -> BoxFuture<'_, Result<()>>;

    fn refresh_album_search(&self, album_id: i64) -> BoxFuture<'_, Result<()>>;

    fn refresh_artist_search(&self, artist_id: i64) -> BoxFuture<'_, Result<()>>;

    fn refresh_event_search(&self, event_id: i64) -> BoxFuture<'_, Result<()>>;

    fn discard_unknown_events(&self) -> BoxFuture<'_, Result<()>>;

    fn repair_event_dates_and_artwork(&self) -> BoxFuture<'_, Result<()>>;

    fn rebuild_relations(&self) -> BoxFuture<'_, Result<()>>;

    fn auto_merge(&self) -> BoxFuture<'_, Result<usize>>;
}

pub struct NewTrack<'a> {
    pub title: &'a str,
    pub album_id: Option<i64>,
    pub event_id: Option<i64>,
    pub cue_track_no: Option<i64>,
    pub disc_no: Option<i64>,
    pub track_no: Option<i64>,
    pub duration_ms: Option<i64>,
    pub date: Option<&'a str>,
    pub year: Option<i64>,
    pub artwork_id: Option<i64>,
}

pub struct NewTrackAudioSource<'a> {
    pub kind: &'a str,
    pub media_file_id: i64,
    pub cue_sheet_id: Option<i64>,
    pub codec: &'a str,
    pub sample_rate: Option<i64>,
    pub start_sample: Option<i64>,
    pub end_sample: Option<i64>,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
    pub renderer: &'a str,
}
