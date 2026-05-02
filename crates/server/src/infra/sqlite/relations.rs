use crate::application::relations::RelationRepository;
use crate::domain::{ArtistId, RelationGraph};
use crate::infra::sqlite::db;
use anyhow::Result;
use easy_musiclib_shared as api;
use futures::FutureExt;
use futures::future::BoxFuture;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct SqliteRelationRepository {
    pool: SqlitePool,
}

impl SqliteRelationRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl RelationRepository for SqliteRelationRepository {
    fn resolve_artist_id<'a>(&'a self, ident: &'a str) -> BoxFuture<'a, Result<ArtistId>> {
        async move {
            db::resolve_id(&self.pool, "artists", ident)
                .await
                .map(ArtistId::new)
        }
        .boxed()
    }

    fn relation_graph(
        &self,
        artist_id: Option<ArtistId>,
        depth: i64,
        limit_nodes: i64,
    ) -> BoxFuture<'_, Result<RelationGraph>> {
        async move {
            let graph: api::RelationGraph =
                db::relation_graph(&self.pool, artist_id.map(ArtistId::raw), depth, limit_nodes)
                    .await?;
            Ok(graph.into())
        }
        .boxed()
    }

    fn rebuild_relations(&self) -> BoxFuture<'_, Result<()>> {
        async move { db::rebuild_relations(&self.pool).await }.boxed()
    }
}
