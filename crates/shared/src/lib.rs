use serde::{Deserialize, Serialize};

pub type Id = i64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRef {
    pub id: Id,
    pub uuid: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackSummary {
    pub id: Id,
    pub uuid: String,
    pub title: String,
    pub album: Option<EntityRef>,
    pub artists: Vec<EntityRef>,
    pub event: Option<EntityRef>,
    pub artwork_id: Option<Id>,
    pub track_no: Option<i64>,
    pub disc_no: Option<i64>,
    pub duration_ms: Option<i64>,
    pub year: Option<i64>,
    pub date: Option<String>,
    pub liked_at: Option<i64>,
    pub is_cue: bool,
    pub playable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackDetail {
    #[serde(flatten)]
    pub summary: TrackSummary,
    pub file_path: Option<String>,
    pub renderer: Option<String>,
    pub cue_track_no: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumSummary {
    pub id: Id,
    pub uuid: String,
    pub title: String,
    pub album_artists: Vec<EntityRef>,
    pub event: Option<EntityRef>,
    pub artwork_id: Option<Id>,
    pub year: Option<i64>,
    pub date: Option<String>,
    pub liked_at: Option<i64>,
    pub song_count: i64,
    pub disc_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumDetail {
    #[serde(flatten)]
    pub summary: AlbumSummary,
    pub tracks: Vec<TrackSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistSummary {
    pub id: Id,
    pub uuid: String,
    pub name: String,
    pub artwork_id: Option<Id>,
    pub liked_at: Option<i64>,
    pub album_count: i64,
    pub track_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistDetail {
    #[serde(flatten)]
    pub summary: ArtistSummary,
    pub albums: Vec<AlbumSummary>,
    pub tracks: Vec<TrackSummary>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSummary {
    pub id: Id,
    pub uuid: String,
    pub name: String,
    pub year: Option<i64>,
    pub date: Option<String>,
    pub liked_at: Option<i64>,
    pub album_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDetail {
    #[serde(flatten)]
    pub summary: EventSummary,
    pub albums: Vec<AlbumSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub tracks: Vec<TrackSummary>,
    pub albums: Vec<AlbumSummary>,
    pub artists: Vec<ArtistSummary>,
    pub events: Vec<EventSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationNode {
    pub id: Id,
    pub uuid: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationEdge {
    pub source: Id,
    pub target: Id,
    pub strength: i64,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationGraph {
    pub nodes: Vec<RelationNode>,
    pub edges: Vec<RelationEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsCandidate {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_ms: Option<i64>,
    pub lyrics: String,
    pub score: f64,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanJobRequest {
    pub roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanJobStatus {
    pub id: Id,
    pub status: String,
    pub root_paths: Vec<String>,
    pub total_files: i64,
    pub scanned_files: i64,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowserPlaybackFormat {
    #[serde(rename = "opus_256k")]
    Opus256k,
    #[serde(rename = "flac_48k")]
    Flac48k,
}

impl Default for BrowserPlaybackFormat {
    fn default() -> Self {
        Self::Opus256k
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub browser_playback_format: BrowserPlaybackFormat,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            browser_playback_format: BrowserPlaybackFormat::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAppSettingsRequest {
    pub browser_playback_format: BrowserPlaybackFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlsCacheClearResponse {
    pub cache_dir: String,
    pub removed_files: u64,
    pub removed_dirs: u64,
    pub removed_bytes: u64,
    pub skipped_active_generators: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LikePatch {
    pub liked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateArtistRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddAliasRequest {
    pub alias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeArtistsRequest {
    pub target: String,
    pub source: String,
    #[serde(default)]
    pub by_name: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliasCsvImportRequest {
    pub csv: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    pub message: String,
}
