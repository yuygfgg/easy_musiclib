use crate::domain::{ArtistId, RelationGraph};
use anyhow::Result;
use futures::future::BoxFuture;

pub trait RelationRepository: Send + Sync {
    fn resolve_artist_id<'a>(&'a self, ident: &'a str) -> BoxFuture<'a, Result<ArtistId>>;

    fn relation_graph(
        &self,
        artist_id: Option<ArtistId>,
        depth: i64,
        limit_nodes: i64,
    ) -> BoxFuture<'_, Result<RelationGraph>>;

    fn rebuild_relations(&self) -> BoxFuture<'_, Result<()>>;
}

pub async fn relation_graph(
    repository: &impl RelationRepository,
    artist_ident: Option<&str>,
    depth: i64,
    limit_nodes: i64,
) -> Result<RelationGraph> {
    let artist_id = match artist_ident {
        Some(artist_ident) => Some(repository.resolve_artist_id(artist_ident).await?),
        None => None,
    };
    repository
        .relation_graph(artist_id, depth, limit_nodes)
        .await
}

pub async fn rebuild_relations(repository: &impl RelationRepository) -> Result<()> {
    repository.rebuild_relations().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Name, RelationEdge, RelationNode};
    use futures::FutureExt;
    use std::sync::{Arc, Mutex};

    struct FakeRepo {
        resolved: ArtistId,
        requested: Arc<Mutex<Option<Option<ArtistId>>>>,
    }

    impl RelationRepository for FakeRepo {
        fn resolve_artist_id<'a>(&'a self, _ident: &'a str) -> BoxFuture<'a, Result<ArtistId>> {
            async move { Ok(self.resolved) }.boxed()
        }

        fn relation_graph(
            &self,
            artist_id: Option<ArtistId>,
            _depth: i64,
            _limit_nodes: i64,
        ) -> BoxFuture<'_, Result<RelationGraph>> {
            async move {
                *self.requested.lock().unwrap() = Some(artist_id);
                Ok(RelationGraph {
                    nodes: vec![RelationNode {
                        id: ArtistId::new(1),
                        uuid: "artist-1".to_string(),
                        name: Name::new("Artist"),
                    }],
                    edges: vec![RelationEdge {
                        source: ArtistId::new(1),
                        target: ArtistId::new(2),
                        strength: 3,
                        details: vec!["co-artist".to_string()],
                    }],
                })
            }
            .boxed()
        }

        fn rebuild_relations(&self) -> BoxFuture<'_, Result<()>> {
            async move { Ok(()) }.boxed()
        }
    }

    #[tokio::test]
    async fn resolves_artist_identifier_before_querying_graph() {
        let requested = Arc::new(Mutex::new(None));
        let repo = FakeRepo {
            resolved: ArtistId::new(42),
            requested: requested.clone(),
        };

        relation_graph(&repo, Some("alice"), 2, 500).await.unwrap();

        assert_eq!(*requested.lock().unwrap(), Some(Some(ArtistId::new(42))));
    }
}
