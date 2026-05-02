use crate::application::catalog::{
    CatalogEntityKind, CatalogRepository, ListAlbumsInput, ListArtistsInput, ListEventsInput,
    ListTracksInput,
};
use crate::domain::{
    AlbumDetail, AlbumId, AlbumSummary, ArtistDetail, ArtistId, ArtistSummary, EntityId,
    EventDetail, EventId, EventSummary, ListPage, SearchResult, TrackDetail, TrackId, TrackSummary,
};
use crate::infra::sqlite::db;
use anyhow::Result;
use futures::FutureExt;
use futures::future::BoxFuture;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct SqliteCatalogRepository {
    pool: SqlitePool,
}

impl SqliteCatalogRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl CatalogRepository for SqliteCatalogRepository {
    fn resolve_id(
        &self,
        kind: CatalogEntityKind,
        ident: String,
    ) -> BoxFuture<'_, Result<EntityId>> {
        async move {
            db::resolve_id(&self.pool, kind.as_str(), &ident)
                .await
                .map(EntityId::new)
        }
        .boxed()
    }

    fn list_tracks(&self, input: ListTracksInput) -> BoxFuture<'_, Result<ListPage<TrackSummary>>> {
        async move {
            Ok(db::list_tracks(
                &self.pool,
                input.cursor,
                input.offset,
                input.limit,
                input.artist_id.map(ArtistId::raw),
                input.album_id.map(AlbumId::raw),
                input.event_id.map(EventId::raw),
                input.liked,
                input.q,
            )
            .await?
            .into())
        }
        .boxed()
    }

    fn fetch_track_detail(&self, id: TrackId) -> BoxFuture<'_, Result<TrackDetail>> {
        async move {
            db::fetch_track_detail(&self.pool, id.raw())
                .await
                .map(Into::into)
        }
        .boxed()
    }

    fn list_albums(&self, input: ListAlbumsInput) -> BoxFuture<'_, Result<ListPage<AlbumSummary>>> {
        async move {
            Ok(db::list_albums(
                &self.pool,
                input.cursor,
                input.offset,
                input.limit,
                input.artist_id.map(ArtistId::raw),
                input.event_id.map(EventId::raw),
                input.liked,
                input.q,
            )
            .await?
            .into())
        }
        .boxed()
    }

    fn fetch_album_detail(&self, id: AlbumId) -> BoxFuture<'_, Result<AlbumDetail>> {
        async move {
            db::fetch_album_detail(&self.pool, id.raw())
                .await
                .map(Into::into)
        }
        .boxed()
    }

    fn list_artists(
        &self,
        input: ListArtistsInput,
    ) -> BoxFuture<'_, Result<ListPage<ArtistSummary>>> {
        async move {
            Ok(db::list_artists(
                &self.pool,
                input.cursor,
                input.offset,
                input.limit,
                input.liked,
                input.q,
            )
            .await?
            .into())
        }
        .boxed()
    }

    fn fetch_artist_detail(&self, id: ArtistId) -> BoxFuture<'_, Result<ArtistDetail>> {
        async move {
            db::fetch_artist_detail(&self.pool, id.raw())
                .await
                .map(Into::into)
        }
        .boxed()
    }

    fn list_events(&self, input: ListEventsInput) -> BoxFuture<'_, Result<ListPage<EventSummary>>> {
        async move {
            Ok(db::list_events(
                &self.pool,
                input.cursor,
                input.offset,
                input.limit,
                input.liked,
                input.q,
            )
            .await?
            .into())
        }
        .boxed()
    }

    fn fetch_event_detail(&self, id: EventId) -> BoxFuture<'_, Result<EventDetail>> {
        async move {
            db::fetch_event_detail(&self.pool, id.raw())
                .await
                .map(Into::into)
        }
        .boxed()
    }

    fn search(&self, q: String, limit: i64) -> BoxFuture<'_, Result<SearchResult>> {
        async move { db::search(&self.pool, &q, limit).await.map(Into::into) }.boxed()
    }

    fn set_liked(
        &self,
        kind: CatalogEntityKind,
        id: EntityId,
        liked: bool,
    ) -> BoxFuture<'_, Result<()>> {
        async move { db::set_liked(&self.pool, kind.as_str(), id.raw(), liked).await }.boxed()
    }
}
