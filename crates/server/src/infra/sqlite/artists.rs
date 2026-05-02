use crate::application::artists::ArtistRepository;
use crate::domain::{ArtistId, ArtistSummary, ArtworkId};
use crate::infra::sqlite::db;
use anyhow::Result;
use easy_musiclib_shared as api;
use futures::FutureExt;
use futures::future::BoxFuture;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct SqliteArtistRepository {
    pool: SqlitePool,
}

impl SqliteArtistRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl ArtistRepository for SqliteArtistRepository {
    fn resolve_artist_id<'a>(&'a self, ident: &'a str) -> BoxFuture<'a, Result<ArtistId>> {
        async move {
            db::resolve_id(&self.pool, "artists", ident)
                .await
                .map(ArtistId::new)
        }
        .boxed()
    }

    fn create_artist<'a>(&'a self, name: &'a str) -> BoxFuture<'a, Result<ArtistSummary>> {
        async move {
            let summary: api::ArtistSummary = db::create_artist(&self.pool, name).await?;
            Ok(summary.into())
        }
        .boxed()
    }

    fn ensure_artist<'a>(
        &'a self,
        name: &'a str,
        artwork_id: Option<ArtworkId>,
    ) -> BoxFuture<'a, Result<ArtistId>> {
        async move {
            db::ensure_artist(&self.pool, name, artwork_id.map(ArtworkId::raw))
                .await
                .map(ArtistId::new)
        }
        .boxed()
    }

    fn add_artist_alias<'a>(
        &'a self,
        artist_id: ArtistId,
        alias: &'a str,
    ) -> BoxFuture<'a, Result<()>> {
        async move { db::add_artist_alias(&self.pool, artist_id.raw(), alias).await }.boxed()
    }

    fn merge_artists<'a>(
        &'a self,
        target_id: ArtistId,
        source_id: ArtistId,
        reason: &'a str,
    ) -> BoxFuture<'a, Result<()>> {
        async move { db::merge_artists(&self.pool, target_id.raw(), source_id.raw(), reason).await }
            .boxed()
    }

    fn auto_merge(&self) -> BoxFuture<'_, Result<usize>> {
        async move { db::auto_merge(&self.pool).await }.boxed()
    }

    fn import_alias_csv<'a>(&'a self, csv: &'a str) -> BoxFuture<'a, Result<usize>> {
        async move { db::import_alias_csv(&self.pool, csv).await }.boxed()
    }
}
