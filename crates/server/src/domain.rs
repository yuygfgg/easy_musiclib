macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(i64);

        impl $name {
            pub fn new(value: i64) -> Self {
                Self(value)
            }

            pub fn raw(self) -> i64 {
                self.0
            }
        }

        impl From<i64> for $name {
            fn from(value: i64) -> Self {
                Self::new(value)
            }
        }

        impl From<$name> for i64 {
            fn from(value: $name) -> Self {
                value.raw()
            }
        }
    };
}

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

#[derive(Debug, Clone)]
pub struct EntityRef {
    pub id: EntityId,
    pub uuid: String,
    pub name: Name,
}

#[derive(Debug, Clone)]
pub struct ListPage<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<EntityId>,
    pub total: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct TrackSummary {
    pub id: TrackId,
    pub uuid: String,
    pub title: String,
    pub album: Option<EntityRef>,
    pub artists: Vec<EntityRef>,
    pub event: Option<EntityRef>,
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

#[derive(Debug, Clone)]
pub struct TrackDetail {
    pub summary: TrackSummary,
    pub file_path: Option<String>,
    pub renderer: Option<String>,
    pub cue_track_no: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct AlbumSummary {
    pub id: AlbumId,
    pub uuid: String,
    pub title: String,
    pub album_artists: Vec<EntityRef>,
    pub event: Option<EntityRef>,
    pub artwork_id: Option<ArtworkId>,
    pub year: Option<i64>,
    pub date: Option<String>,
    pub liked_at: Option<i64>,
    pub song_count: i64,
}

#[derive(Debug, Clone)]
pub struct AlbumDetail {
    pub summary: AlbumSummary,
    pub tracks: Vec<TrackSummary>,
}

#[derive(Debug, Clone)]
pub struct ArtistSummary {
    pub id: ArtistId,
    pub uuid: String,
    pub name: Name,
    pub artwork_id: Option<ArtworkId>,
    pub liked_at: Option<i64>,
    pub album_count: i64,
    pub track_count: i64,
}

#[derive(Debug, Clone)]
pub struct ArtistDetail {
    pub summary: ArtistSummary,
    pub albums: Vec<AlbumSummary>,
    pub tracks: Vec<TrackSummary>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EventSummary {
    pub id: EventId,
    pub uuid: String,
    pub name: Name,
    pub year: Option<i64>,
    pub date: Option<String>,
    pub liked_at: Option<i64>,
    pub album_count: i64,
}

#[derive(Debug, Clone)]
pub struct EventDetail {
    pub summary: EventSummary,
    pub albums: Vec<AlbumSummary>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub tracks: Vec<TrackSummary>,
    pub albums: Vec<AlbumSummary>,
    pub artists: Vec<ArtistSummary>,
    pub events: Vec<EventSummary>,
}

#[derive(Debug, Clone)]
pub struct RelationNode {
    pub id: ArtistId,
    pub uuid: String,
    pub name: Name,
}

#[derive(Debug, Clone)]
pub struct RelationEdge {
    pub source: ArtistId,
    pub target: ArtistId,
    pub strength: i64,
    pub details: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RelationGraph {
    pub nodes: Vec<RelationNode>,
    pub edges: Vec<RelationEdge>,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone)]
pub struct ScanJob {
    pub id: ScanJobId,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserPlaybackFormat {
    Opus256k,
    Flac48k,
}

impl Default for BrowserPlaybackFormat {
    fn default() -> Self {
        Self::Opus256k
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct UpdateAppSettings {
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
