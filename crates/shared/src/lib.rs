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
    #[serde(rename = "opus")]
    Opus,
    #[serde(rename = "flac")]
    Flac,
}

pub const DEFAULT_BROWSER_PLAYBACK_OPUS_BITRATE: i64 = 256_000;
pub const BROWSER_PLAYBACK_OPUS_BITRATE_OPTIONS: [i64; 7] =
    [64_000, 96_000, 128_000, 160_000, 192_000, 256_000, 320_000];
pub const MIN_BROWSER_PLAYBACK_OPUS_BITRATE: i64 = 64_000;
pub const MAX_BROWSER_PLAYBACK_OPUS_BITRATE: i64 = 320_000;
pub const DEFAULT_BROWSER_PLAYBACK_FLAC_SAMPLE_RATE: i64 = 48_000;
pub const BROWSER_PLAYBACK_FLAC_SAMPLE_RATE_OPTIONS: [i64; 6] =
    [44_100, 48_000, 88_200, 96_000, 176_400, 192_000];
pub const MIN_BROWSER_PLAYBACK_FLAC_SAMPLE_RATE: i64 = 44_100;
pub const MAX_BROWSER_PLAYBACK_FLAC_SAMPLE_RATE: i64 = 192_000;

pub fn normalize_browser_playback_opus_bitrate(value: i64) -> i64 {
    nearest_browser_playback_option(
        value,
        &BROWSER_PLAYBACK_OPUS_BITRATE_OPTIONS,
        DEFAULT_BROWSER_PLAYBACK_OPUS_BITRATE,
    )
}

pub fn normalize_browser_playback_flac_sample_rate(value: i64) -> i64 {
    nearest_browser_playback_option(
        value,
        &BROWSER_PLAYBACK_FLAC_SAMPLE_RATE_OPTIONS,
        DEFAULT_BROWSER_PLAYBACK_FLAC_SAMPLE_RATE,
    )
}

fn nearest_browser_playback_option(value: i64, options: &[i64], default: i64) -> i64 {
    options
        .iter()
        .copied()
        .min_by_key(|option| (i128::from(value) - i128::from(*option)).abs())
        .unwrap_or(default)
}

impl Default for BrowserPlaybackFormat {
    fn default() -> Self {
        Self::Opus
    }
}

fn default_browser_playback_opus_bitrate() -> i64 {
    DEFAULT_BROWSER_PLAYBACK_OPUS_BITRATE
}

fn default_browser_playback_flac_sample_rate() -> i64 {
    DEFAULT_BROWSER_PLAYBACK_FLAC_SAMPLE_RATE
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BrowserPlaybackSettings {
    #[serde(default)]
    pub format: BrowserPlaybackFormat,
    #[serde(default = "default_browser_playback_opus_bitrate")]
    pub opus_bitrate: i64,
    #[serde(default = "default_browser_playback_flac_sample_rate")]
    pub flac_sample_rate: i64,
}

impl Default for BrowserPlaybackSettings {
    fn default() -> Self {
        Self {
            format: BrowserPlaybackFormat::default(),
            opus_bitrate: DEFAULT_BROWSER_PLAYBACK_OPUS_BITRATE,
            flac_sample_rate: DEFAULT_BROWSER_PLAYBACK_FLAC_SAMPLE_RATE,
        }
    }
}

impl BrowserPlaybackSettings {
    pub fn normalized(mut self) -> Self {
        self.opus_bitrate = normalize_browser_playback_opus_bitrate(self.opus_bitrate);
        self.flac_sample_rate = normalize_browser_playback_flac_sample_rate(self.flac_sample_rate);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub browser_playback: BrowserPlaybackSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            browser_playback: BrowserPlaybackSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAppSettingsRequest {
    #[serde(default)]
    pub browser_playback: BrowserPlaybackSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatusResponse {
    pub login_required: bool,
    pub authenticated: bool,
    pub username: Option<String>,
    pub secure_transport: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSummary {
    pub username: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountListResponse {
    pub accounts: Vec<AccountSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccountRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAccountPasswordRequest {
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteAccountResponse {
    pub deleted: bool,
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
