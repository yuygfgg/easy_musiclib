use crate::domain::{
    AlbumDetail, AlbumId, AlbumSummary, ArtistDetail, ArtistId, ArtistSummary, EntityId,
    EventDetail, EventId, EventSummary, ListPage, SearchResult, TrackDetail, TrackId, TrackSummary,
};
use anyhow::Result;
use futures::future::BoxFuture;

#[derive(Debug, Clone, Copy)]
pub enum CatalogEntityKind {
    Tracks,
    Albums,
    Artists,
    Events,
}

impl CatalogEntityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tracks => "tracks",
            Self::Albums => "albums",
            Self::Artists => "artists",
            Self::Events => "events",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ListTracksInput {
    pub cursor: Option<i64>,
    pub offset: Option<i64>,
    pub limit: i64,
    pub artist_id: Option<ArtistId>,
    pub album_id: Option<AlbumId>,
    pub event_id: Option<EventId>,
    pub liked: Option<bool>,
    pub q: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ListAlbumsInput {
    pub cursor: Option<i64>,
    pub offset: Option<i64>,
    pub limit: i64,
    pub artist_id: Option<ArtistId>,
    pub event_id: Option<EventId>,
    pub liked: Option<bool>,
    pub q: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ListArtistsInput {
    pub cursor: Option<i64>,
    pub offset: Option<i64>,
    pub limit: i64,
    pub liked: Option<bool>,
    pub q: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ListEventsInput {
    pub cursor: Option<i64>,
    pub offset: Option<i64>,
    pub limit: i64,
    pub liked: Option<bool>,
    pub q: Option<String>,
}

pub trait CatalogRepository: Send + Sync {
    fn resolve_id(&self, kind: CatalogEntityKind, ident: String)
    -> BoxFuture<'_, Result<EntityId>>;

    fn list_tracks(&self, input: ListTracksInput) -> BoxFuture<'_, Result<ListPage<TrackSummary>>>;

    fn fetch_track_detail(&self, id: TrackId) -> BoxFuture<'_, Result<TrackDetail>>;

    fn list_albums(&self, input: ListAlbumsInput) -> BoxFuture<'_, Result<ListPage<AlbumSummary>>>;

    fn fetch_album_detail(&self, id: AlbumId) -> BoxFuture<'_, Result<AlbumDetail>>;

    fn list_artists(
        &self,
        input: ListArtistsInput,
    ) -> BoxFuture<'_, Result<ListPage<ArtistSummary>>>;

    fn fetch_artist_detail(&self, id: ArtistId) -> BoxFuture<'_, Result<ArtistDetail>>;

    fn list_events(&self, input: ListEventsInput) -> BoxFuture<'_, Result<ListPage<EventSummary>>>;

    fn fetch_event_detail(&self, id: EventId) -> BoxFuture<'_, Result<EventDetail>>;

    fn search(&self, q: String, limit: i64) -> BoxFuture<'_, Result<SearchResult>>;

    fn set_liked(
        &self,
        kind: CatalogEntityKind,
        id: EntityId,
        liked: bool,
    ) -> BoxFuture<'_, Result<()>>;
}

pub async fn resolve_id(
    repository: &impl CatalogRepository,
    kind: CatalogEntityKind,
    ident: String,
) -> Result<EntityId> {
    repository.resolve_id(kind, ident).await
}

pub async fn list_tracks(
    repository: &impl CatalogRepository,
    input: ListTracksInput,
) -> Result<ListPage<TrackSummary>> {
    repository.list_tracks(input).await
}

pub async fn fetch_track_detail(
    repository: &impl CatalogRepository,
    id: TrackId,
) -> Result<TrackDetail> {
    repository.fetch_track_detail(id).await
}

pub async fn list_albums(
    repository: &impl CatalogRepository,
    input: ListAlbumsInput,
) -> Result<ListPage<AlbumSummary>> {
    repository.list_albums(input).await
}

pub async fn fetch_album_detail(
    repository: &impl CatalogRepository,
    id: AlbumId,
) -> Result<AlbumDetail> {
    repository.fetch_album_detail(id).await
}

pub async fn list_artists(
    repository: &impl CatalogRepository,
    input: ListArtistsInput,
) -> Result<ListPage<ArtistSummary>> {
    repository.list_artists(input).await
}

pub async fn fetch_artist_detail(
    repository: &impl CatalogRepository,
    id: ArtistId,
) -> Result<ArtistDetail> {
    repository.fetch_artist_detail(id).await
}

pub async fn list_events(
    repository: &impl CatalogRepository,
    input: ListEventsInput,
) -> Result<ListPage<EventSummary>> {
    repository.list_events(input).await
}

pub async fn fetch_event_detail(
    repository: &impl CatalogRepository,
    id: EventId,
) -> Result<EventDetail> {
    repository.fetch_event_detail(id).await
}

pub async fn search(
    repository: &impl CatalogRepository,
    q: String,
    limit: i64,
) -> Result<SearchResult> {
    repository.search(q, limit).await
}

pub async fn set_liked(
    repository: &impl CatalogRepository,
    kind: CatalogEntityKind,
    id: EntityId,
    liked: bool,
) -> Result<()> {
    repository.set_liked(kind, id, liked).await
}
