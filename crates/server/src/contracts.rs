use crate::domain;
use easy_musiclib_shared as api;

impl From<api::EntityRef> for domain::EntityRef {
    fn from(value: api::EntityRef) -> Self {
        Self {
            id: domain::EntityId::new(value.id),
            uuid: value.uuid,
            name: domain::Name::new(value.name),
        }
    }
}

impl From<domain::EntityRef> for api::EntityRef {
    fn from(value: domain::EntityRef) -> Self {
        Self {
            id: value.id.raw(),
            uuid: value.uuid,
            name: value.name.into_string(),
        }
    }
}

impl<T, U> From<api::ListResponse<T>> for domain::ListPage<U>
where
    U: From<T>,
{
    fn from(value: api::ListResponse<T>) -> Self {
        Self {
            items: value.items.into_iter().map(Into::into).collect(),
            next_cursor: value.next_cursor.map(domain::EntityId::new),
            total: value.total,
        }
    }
}

impl<T, U> From<domain::ListPage<T>> for api::ListResponse<U>
where
    U: From<T>,
{
    fn from(value: domain::ListPage<T>) -> Self {
        Self {
            items: value.items.into_iter().map(Into::into).collect(),
            next_cursor: value.next_cursor.map(domain::EntityId::raw),
            total: value.total,
        }
    }
}

impl From<api::TrackSummary> for domain::TrackSummary {
    fn from(value: api::TrackSummary) -> Self {
        Self {
            id: domain::TrackId::new(value.id),
            uuid: value.uuid,
            title: value.title,
            album: value.album.map(Into::into),
            artists: value.artists.into_iter().map(Into::into).collect(),
            event: value.event.map(Into::into),
            artwork_id: value.artwork_id.map(domain::ArtworkId::new),
            track_no: value.track_no,
            disc_no: value.disc_no,
            duration_ms: value.duration_ms,
            year: value.year,
            date: value.date,
            liked_at: value.liked_at,
            is_cue: value.is_cue,
            playable: value.playable,
        }
    }
}

impl From<domain::TrackSummary> for api::TrackSummary {
    fn from(value: domain::TrackSummary) -> Self {
        Self {
            id: value.id.raw(),
            uuid: value.uuid,
            title: value.title,
            album: value.album.map(Into::into),
            artists: value.artists.into_iter().map(Into::into).collect(),
            event: value.event.map(Into::into),
            artwork_id: value.artwork_id.map(domain::ArtworkId::raw),
            track_no: value.track_no,
            disc_no: value.disc_no,
            duration_ms: value.duration_ms,
            year: value.year,
            date: value.date,
            liked_at: value.liked_at,
            is_cue: value.is_cue,
            playable: value.playable,
        }
    }
}

impl From<api::TrackDetail> for domain::TrackDetail {
    fn from(value: api::TrackDetail) -> Self {
        Self {
            summary: value.summary.into(),
            file_path: value.file_path,
            renderer: value.renderer,
            cue_track_no: value.cue_track_no,
        }
    }
}

impl From<domain::TrackDetail> for api::TrackDetail {
    fn from(value: domain::TrackDetail) -> Self {
        Self {
            summary: value.summary.into(),
            file_path: value.file_path,
            renderer: value.renderer,
            cue_track_no: value.cue_track_no,
        }
    }
}

impl From<api::AlbumSummary> for domain::AlbumSummary {
    fn from(value: api::AlbumSummary) -> Self {
        Self {
            id: domain::AlbumId::new(value.id),
            uuid: value.uuid,
            title: value.title,
            album_artists: value.album_artists.into_iter().map(Into::into).collect(),
            event: value.event.map(Into::into),
            artwork_id: value.artwork_id.map(domain::ArtworkId::new),
            year: value.year,
            date: value.date,
            liked_at: value.liked_at,
            song_count: value.song_count,
        }
    }
}

impl From<domain::AlbumSummary> for api::AlbumSummary {
    fn from(value: domain::AlbumSummary) -> Self {
        Self {
            id: value.id.raw(),
            uuid: value.uuid,
            title: value.title,
            album_artists: value.album_artists.into_iter().map(Into::into).collect(),
            event: value.event.map(Into::into),
            artwork_id: value.artwork_id.map(domain::ArtworkId::raw),
            year: value.year,
            date: value.date,
            liked_at: value.liked_at,
            song_count: value.song_count,
        }
    }
}

impl From<api::AlbumDetail> for domain::AlbumDetail {
    fn from(value: api::AlbumDetail) -> Self {
        Self {
            summary: value.summary.into(),
            tracks: value.tracks.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<domain::AlbumDetail> for api::AlbumDetail {
    fn from(value: domain::AlbumDetail) -> Self {
        Self {
            summary: value.summary.into(),
            tracks: value.tracks.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<api::ArtistSummary> for domain::ArtistSummary {
    fn from(value: api::ArtistSummary) -> Self {
        Self {
            id: domain::ArtistId::new(value.id),
            uuid: value.uuid,
            name: domain::Name::new(value.name),
            artwork_id: value.artwork_id.map(domain::ArtworkId::new),
            liked_at: value.liked_at,
            album_count: value.album_count,
            track_count: value.track_count,
        }
    }
}

impl From<domain::ArtistSummary> for api::ArtistSummary {
    fn from(value: domain::ArtistSummary) -> Self {
        Self {
            id: value.id.raw(),
            uuid: value.uuid,
            name: value.name.into_string(),
            artwork_id: value.artwork_id.map(domain::ArtworkId::raw),
            liked_at: value.liked_at,
            album_count: value.album_count,
            track_count: value.track_count,
        }
    }
}

impl From<api::ArtistDetail> for domain::ArtistDetail {
    fn from(value: api::ArtistDetail) -> Self {
        Self {
            summary: value.summary.into(),
            albums: value.albums.into_iter().map(Into::into).collect(),
            tracks: value.tracks.into_iter().map(Into::into).collect(),
            aliases: value.aliases,
        }
    }
}

impl From<domain::ArtistDetail> for api::ArtistDetail {
    fn from(value: domain::ArtistDetail) -> Self {
        Self {
            summary: value.summary.into(),
            albums: value.albums.into_iter().map(Into::into).collect(),
            tracks: value.tracks.into_iter().map(Into::into).collect(),
            aliases: value.aliases,
        }
    }
}

impl From<api::EventSummary> for domain::EventSummary {
    fn from(value: api::EventSummary) -> Self {
        Self {
            id: domain::EventId::new(value.id),
            uuid: value.uuid,
            name: domain::Name::new(value.name),
            year: value.year,
            date: value.date,
            liked_at: value.liked_at,
            album_count: value.album_count,
        }
    }
}

impl From<domain::EventSummary> for api::EventSummary {
    fn from(value: domain::EventSummary) -> Self {
        Self {
            id: value.id.raw(),
            uuid: value.uuid,
            name: value.name.into_string(),
            year: value.year,
            date: value.date,
            liked_at: value.liked_at,
            album_count: value.album_count,
        }
    }
}

impl From<api::EventDetail> for domain::EventDetail {
    fn from(value: api::EventDetail) -> Self {
        Self {
            summary: value.summary.into(),
            albums: value.albums.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<domain::EventDetail> for api::EventDetail {
    fn from(value: domain::EventDetail) -> Self {
        Self {
            summary: value.summary.into(),
            albums: value.albums.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<api::SearchResponse> for domain::SearchResult {
    fn from(value: api::SearchResponse) -> Self {
        Self {
            tracks: value.tracks.into_iter().map(Into::into).collect(),
            albums: value.albums.into_iter().map(Into::into).collect(),
            artists: value.artists.into_iter().map(Into::into).collect(),
            events: value.events.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<domain::SearchResult> for api::SearchResponse {
    fn from(value: domain::SearchResult) -> Self {
        Self {
            tracks: value.tracks.into_iter().map(Into::into).collect(),
            albums: value.albums.into_iter().map(Into::into).collect(),
            artists: value.artists.into_iter().map(Into::into).collect(),
            events: value.events.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<api::RelationGraph> for domain::RelationGraph {
    fn from(value: api::RelationGraph) -> Self {
        Self {
            nodes: value.nodes.into_iter().map(Into::into).collect(),
            edges: value.edges.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<domain::RelationGraph> for api::RelationGraph {
    fn from(value: domain::RelationGraph) -> Self {
        Self {
            nodes: value.nodes.into_iter().map(Into::into).collect(),
            edges: value.edges.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<api::RelationNode> for domain::RelationNode {
    fn from(value: api::RelationNode) -> Self {
        Self {
            id: domain::ArtistId::new(value.id),
            uuid: value.uuid,
            name: domain::Name::new(value.name),
        }
    }
}

impl From<domain::RelationNode> for api::RelationNode {
    fn from(value: domain::RelationNode) -> Self {
        Self {
            id: value.id.raw(),
            uuid: value.uuid,
            name: value.name.into_string(),
        }
    }
}

impl From<api::RelationEdge> for domain::RelationEdge {
    fn from(value: api::RelationEdge) -> Self {
        Self {
            source: domain::ArtistId::new(value.source),
            target: domain::ArtistId::new(value.target),
            strength: value.strength,
            details: value.details,
        }
    }
}

impl From<domain::RelationEdge> for api::RelationEdge {
    fn from(value: domain::RelationEdge) -> Self {
        Self {
            source: value.source.raw(),
            target: value.target.raw(),
            strength: value.strength,
            details: value.details,
        }
    }
}

impl From<api::LyricsCandidate> for domain::LyricsCandidate {
    fn from(value: api::LyricsCandidate) -> Self {
        Self {
            title: value.title,
            artist: value.artist,
            album: value.album,
            duration_ms: value.duration_ms,
            lyrics: value.lyrics,
            score: value.score,
            provider: value.provider,
        }
    }
}

impl From<domain::LyricsCandidate> for api::LyricsCandidate {
    fn from(value: domain::LyricsCandidate) -> Self {
        Self {
            title: value.title,
            artist: value.artist,
            album: value.album,
            duration_ms: value.duration_ms,
            lyrics: value.lyrics,
            score: value.score,
            provider: value.provider,
        }
    }
}

impl From<api::ScanJobStatus> for domain::ScanJob {
    fn from(value: api::ScanJobStatus) -> Self {
        Self {
            id: domain::ScanJobId::new(value.id),
            state: domain::ScanJobState::from(value.status.as_str()),
            root_paths: value.root_paths,
            total_files: value.total_files,
            scanned_files: value.scanned_files,
            started_at: value.started_at,
            finished_at: value.finished_at,
            error: value.error,
        }
    }
}

impl From<domain::ScanJob> for api::ScanJobStatus {
    fn from(value: domain::ScanJob) -> Self {
        Self {
            id: value.id.raw(),
            status: value.state.as_str().to_string(),
            root_paths: value.root_paths,
            total_files: value.total_files,
            scanned_files: value.scanned_files,
            started_at: value.started_at,
            finished_at: value.finished_at,
            error: value.error,
        }
    }
}

impl From<api::BrowserPlaybackFormat> for domain::BrowserPlaybackFormat {
    fn from(value: api::BrowserPlaybackFormat) -> Self {
        match value {
            api::BrowserPlaybackFormat::Opus256k => Self::Opus256k,
            api::BrowserPlaybackFormat::Flac48k => Self::Flac48k,
        }
    }
}

impl From<domain::BrowserPlaybackFormat> for api::BrowserPlaybackFormat {
    fn from(value: domain::BrowserPlaybackFormat) -> Self {
        match value {
            domain::BrowserPlaybackFormat::Opus256k => Self::Opus256k,
            domain::BrowserPlaybackFormat::Flac48k => Self::Flac48k,
        }
    }
}

impl From<api::AppSettings> for domain::AppSettings {
    fn from(value: api::AppSettings) -> Self {
        Self {
            browser_playback_format: value.browser_playback_format.into(),
        }
    }
}

impl From<domain::AppSettings> for api::AppSettings {
    fn from(value: domain::AppSettings) -> Self {
        Self {
            browser_playback_format: value.browser_playback_format.into(),
        }
    }
}

impl From<api::UpdateAppSettingsRequest> for domain::UpdateAppSettings {
    fn from(value: api::UpdateAppSettingsRequest) -> Self {
        Self {
            browser_playback_format: value.browser_playback_format.into(),
        }
    }
}
