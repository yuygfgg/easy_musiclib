use easy_musiclib_macros::id_type;
use easy_musiclib_shared as api;
use o2o::o2o;

id_type!(EntityId);
id_type!(TrackId);
id_type!(AlbumId);
id_type!(ArtistId);
id_type!(EventId);
id_type!(ArtworkId);
id_type!(MediaFileId);
id_type!(ScanJobId);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Name(String);

impl Name {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for Name {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for Name {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<Name> for String {
    fn from(value: Name) -> Self {
        value.into_string()
    }
}

#[derive(Debug, Clone, o2o)]
#[map_owned(api::EntityRef)]
pub struct EntityRef {
    #[map(~.into())]
    pub id: EntityId,
    pub uuid: String,
    #[map(~.into())]
    pub name: Name,
}

#[derive(Debug, Clone)]
pub struct ListPage<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<EntityId>,
    pub total: Option<i64>,
}

#[derive(Debug, Clone, o2o)]
#[map_owned(api::TrackSummary)]
pub struct TrackSummary {
    #[map(~.into())]
    pub id: TrackId,
    pub uuid: String,
    pub title: String,
    #[map(~.map(Into::into))]
    pub album: Option<EntityRef>,
    #[map(~.into_iter().map(Into::into).collect())]
    pub artists: Vec<EntityRef>,
    #[map(~.map(Into::into))]
    pub event: Option<EntityRef>,
    #[map(~.map(Into::into))]
    pub artwork_id: Option<ArtworkId>,
    pub track_no: Option<i64>,
    pub disc_no: Option<i64>,
    pub duration_ms: Option<i64>,
    pub year: Option<i64>,
    pub date: Option<String>,
    pub liked_at: Option<i64>,
    pub is_cue: bool,
    pub playable: bool,
}

#[derive(Debug, Clone, o2o)]
#[map_owned(api::TrackDetail)]
pub struct TrackDetail {
    #[map(~.into())]
    pub summary: TrackSummary,
    pub file_path: Option<String>,
    pub renderer: Option<String>,
    pub cue_track_no: Option<i64>,
}

#[derive(Debug, Clone, o2o)]
#[map_owned(api::AlbumSummary)]
pub struct AlbumSummary {
    #[map(~.into())]
    pub id: AlbumId,
    pub uuid: String,
    pub title: String,
    #[map(~.into_iter().map(Into::into).collect())]
    pub album_artists: Vec<EntityRef>,
    #[map(~.map(Into::into))]
    pub event: Option<EntityRef>,
    #[map(~.map(Into::into))]
    pub artwork_id: Option<ArtworkId>,
    pub year: Option<i64>,
    pub date: Option<String>,
    pub liked_at: Option<i64>,
    pub song_count: i64,
    pub disc_count: i64,
}

#[derive(Debug, Clone, o2o)]
#[map_owned(api::AlbumDetail)]
pub struct AlbumDetail {
    #[map(~.into())]
    pub summary: AlbumSummary,
    #[map(~.into_iter().map(Into::into).collect())]
    pub tracks: Vec<TrackSummary>,
}

#[derive(Debug, Clone, o2o)]
#[map_owned(api::ArtistSummary)]
pub struct ArtistSummary {
    #[map(~.into())]
    pub id: ArtistId,
    pub uuid: String,
    #[map(~.into())]
    pub name: Name,
    #[map(~.map(Into::into))]
    pub artwork_id: Option<ArtworkId>,
    pub liked_at: Option<i64>,
    pub album_count: i64,
    pub track_count: i64,
}

#[derive(Debug, Clone, o2o)]
#[map_owned(api::ArtistDetail)]
pub struct ArtistDetail {
    #[map(~.into())]
    pub summary: ArtistSummary,
    #[map(~.into_iter().map(Into::into).collect())]
    pub albums: Vec<AlbumSummary>,
    #[map(~.into_iter().map(Into::into).collect())]
    pub tracks: Vec<TrackSummary>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, o2o)]
#[map_owned(api::EventSummary)]
pub struct EventSummary {
    #[map(~.into())]
    pub id: EventId,
    pub uuid: String,
    #[map(~.into())]
    pub name: Name,
    pub year: Option<i64>,
    pub date: Option<String>,
    pub liked_at: Option<i64>,
    pub album_count: i64,
}

#[derive(Debug, Clone, o2o)]
#[map_owned(api::EventDetail)]
pub struct EventDetail {
    #[map(~.into())]
    pub summary: EventSummary,
    #[map(~.into_iter().map(Into::into).collect())]
    pub albums: Vec<AlbumSummary>,
}

#[derive(Debug, Clone, o2o)]
#[map_owned(api::SearchResponse)]
pub struct SearchResult {
    #[map(~.into_iter().map(Into::into).collect())]
    pub tracks: Vec<TrackSummary>,
    #[map(~.into_iter().map(Into::into).collect())]
    pub albums: Vec<AlbumSummary>,
    #[map(~.into_iter().map(Into::into).collect())]
    pub artists: Vec<ArtistSummary>,
    #[map(~.into_iter().map(Into::into).collect())]
    pub events: Vec<EventSummary>,
}

#[derive(Debug, Clone, o2o)]
#[map_owned(api::RelationNode)]
pub struct RelationNode {
    #[map(~.into())]
    pub id: ArtistId,
    pub uuid: String,
    #[map(~.into())]
    pub name: Name,
}

#[derive(Debug, Clone, o2o)]
#[map_owned(api::RelationEdge)]
pub struct RelationEdge {
    #[map(~.into())]
    pub source: ArtistId,
    #[map(~.into())]
    pub target: ArtistId,
    pub strength: i64,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, o2o)]
#[map_owned(api::RelationGraph)]
pub struct RelationGraph {
    #[map(~.into_iter().map(Into::into).collect())]
    pub nodes: Vec<RelationNode>,
    #[map(~.into_iter().map(Into::into).collect())]
    pub edges: Vec<RelationEdge>,
}

#[derive(Debug, Clone, PartialEq, o2o)]
#[map_owned(api::LyricsCandidate)]
pub struct LyricsCandidate {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_ms: Option<i64>,
    pub lyrics: String,
    pub score: f64,
    pub provider: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanJobState {
    Queued,
    Running,
    Completed,
    Failed,
    CancelRequested,
    Other(String),
}

impl ScanJobState {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::CancelRequested => "cancel_requested",
            Self::Other(value) => value.as_str(),
        }
    }
}

impl From<&str> for ScanJobState {
    fn from(value: &str) -> Self {
        match value {
            "queued" => Self::Queued,
            "running" => Self::Running,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "cancel_requested" => Self::CancelRequested,
            value => Self::Other(value.to_string()),
        }
    }
}

impl From<String> for ScanJobState {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<ScanJobState> for String {
    fn from(value: ScanJobState) -> Self {
        value.as_str().to_string()
    }
}

#[derive(Debug, Clone, o2o)]
#[map_owned(api::ScanJobStatus)]
pub struct ScanJob {
    #[map(~.into())]
    pub id: ScanJobId,
    #[map(status, ~.into())]
    pub state: ScanJobState,
    pub root_paths: Vec<String>,
    pub total_files: i64,
    pub scanned_files: i64,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ArtworkSource {
    Sidecar {
        id: ArtworkId,
        path: String,
    },
    Embedded {
        id: ArtworkId,
        media_file_id: Option<MediaFileId>,
        media_path: String,
        picture_index: i64,
        mime: Option<String>,
    },
    Unsupported {
        id: ArtworkId,
        kind: String,
    },
}

impl ArtworkSource {
    pub fn id(&self) -> ArtworkId {
        match self {
            Self::Sidecar { id, .. } | Self::Embedded { id, .. } | Self::Unsupported { id, .. } => {
                *id
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, o2o)]
#[map_owned(api::BrowserPlaybackFormat)]
pub enum BrowserPlaybackFormat {
    Opus256k,
    Flac48k,
}

impl Default for BrowserPlaybackFormat {
    fn default() -> Self {
        Self::Opus256k
    }
}

#[derive(Debug, Clone, o2o)]
#[map_owned(api::AppSettings)]
pub struct AppSettings {
    #[map(~.into())]
    pub browser_playback_format: BrowserPlaybackFormat,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            browser_playback_format: BrowserPlaybackFormat::default(),
        }
    }
}

#[derive(Debug, Clone, o2o)]
#[map_owned(api::UpdateAppSettingsRequest)]
pub struct UpdateAppSettings {
    #[map(~.into())]
    pub browser_playback_format: BrowserPlaybackFormat,
}

#[derive(Debug, Clone)]
pub struct PlaybackSource {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub track_no: Option<i64>,
    pub date: Option<String>,
    pub path: String,
    pub renderer: String,
    pub codec: String,
    pub start_sample: Option<i64>,
    pub end_sample: Option<i64>,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
}
