use crate::domain::{ArtistId, ArtistSummary};
use anyhow::Result;
use futures::future::BoxFuture;

pub trait ArtistRepository: Send + Sync {
    fn resolve_artist_id<'a>(&'a self, ident: &'a str) -> BoxFuture<'a, Result<ArtistId>>;

    fn create_artist<'a>(&'a self, name: &'a str) -> BoxFuture<'a, Result<ArtistSummary>>;

    fn ensure_artist<'a>(
        &'a self,
        name: &'a str,
        artwork_id: Option<crate::domain::ArtworkId>,
    ) -> BoxFuture<'a, Result<ArtistId>>;

    fn add_artist_alias<'a>(
        &'a self,
        artist_id: ArtistId,
        alias: &'a str,
    ) -> BoxFuture<'a, Result<()>>;

    fn merge_artists<'a>(
        &'a self,
        target_id: ArtistId,
        source_id: ArtistId,
        reason: &'a str,
    ) -> BoxFuture<'a, Result<()>>;

    fn auto_merge(&self) -> BoxFuture<'_, Result<usize>>;

    fn import_alias_csv<'a>(&'a self, csv: &'a str) -> BoxFuture<'a, Result<usize>>;
}

pub async fn create_artist(
    repository: &impl ArtistRepository,
    name: &str,
) -> Result<ArtistSummary> {
    repository.create_artist(name).await
}

pub async fn add_artist_alias(
    repository: &impl ArtistRepository,
    artist_ident: &str,
    alias: &str,
) -> Result<()> {
    let artist_id = repository.resolve_artist_id(artist_ident).await?;
    repository.add_artist_alias(artist_id, alias).await
}

pub async fn merge_artists(
    repository: &impl ArtistRepository,
    target: &str,
    source: &str,
    by_name: bool,
    reason: &str,
) -> Result<()> {
    let target_id = if by_name {
        repository.ensure_artist(target, None).await?
    } else {
        repository.resolve_artist_id(target).await?
    };
    let source_id = if by_name {
        match repository.resolve_artist_id(source).await {
            Ok(id) => id,
            Err(_) => repository.ensure_artist(source, None).await?,
        }
    } else {
        repository.resolve_artist_id(source).await?
    };
    repository.merge_artists(target_id, source_id, reason).await
}

pub async fn auto_merge(repository: &impl ArtistRepository) -> Result<usize> {
    repository.auto_merge().await
}

pub async fn import_alias_csv(repository: &impl ArtistRepository, csv: &str) -> Result<usize> {
    repository.import_alias_csv(csv).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ArtistSummary, Name};
    use anyhow::{Result, anyhow};
    use futures::FutureExt;
    use std::sync::{Arc, Mutex};

    struct FakeArtists {
        merges: Arc<Mutex<Vec<(ArtistId, ArtistId, String)>>>,
    }

    impl ArtistRepository for FakeArtists {
        fn resolve_artist_id<'a>(&'a self, ident: &'a str) -> BoxFuture<'a, Result<ArtistId>> {
            async move {
                match ident {
                    "target" => Ok(ArtistId::new(1)),
                    "source" => Ok(ArtistId::new(2)),
                    _ => Err(anyhow!("not found")),
                }
            }
            .boxed()
        }

        fn create_artist<'a>(&'a self, name: &'a str) -> BoxFuture<'a, Result<ArtistSummary>> {
            async move {
                Ok(ArtistSummary {
                    id: ArtistId::new(9),
                    uuid: "artist-9".to_string(),
                    name: Name::new(name),
                    artwork_id: None,
                    liked_at: None,
                    album_count: 0,
                    track_count: 0,
                })
            }
            .boxed()
        }

        fn ensure_artist<'a>(
            &'a self,
            name: &'a str,
            _artwork_id: Option<crate::domain::ArtworkId>,
        ) -> BoxFuture<'a, Result<ArtistId>> {
            async move {
                Ok(match name {
                    "target" => ArtistId::new(1),
                    "new-source" => ArtistId::new(3),
                    _ => ArtistId::new(4),
                })
            }
            .boxed()
        }

        fn add_artist_alias<'a>(
            &'a self,
            _artist_id: ArtistId,
            _alias: &'a str,
        ) -> BoxFuture<'a, Result<()>> {
            async move { Ok(()) }.boxed()
        }

        fn merge_artists<'a>(
            &'a self,
            target_id: ArtistId,
            source_id: ArtistId,
            reason: &'a str,
        ) -> BoxFuture<'a, Result<()>> {
            async move {
                self.merges
                    .lock()
                    .unwrap()
                    .push((target_id, source_id, reason.to_string()));
                Ok(())
            }
            .boxed()
        }

        fn auto_merge(&self) -> BoxFuture<'_, Result<usize>> {
            async move { Ok(0) }.boxed()
        }

        fn import_alias_csv<'a>(&'a self, _csv: &'a str) -> BoxFuture<'a, Result<usize>> {
            async move { Ok(0) }.boxed()
        }
    }

    #[tokio::test]
    async fn merge_by_name_creates_missing_source_before_merge() {
        let merges = Arc::new(Mutex::new(Vec::new()));
        let repo = FakeArtists {
            merges: merges.clone(),
        };

        merge_artists(&repo, "target", "new-source", true, "manual")
            .await
            .unwrap();

        assert_eq!(
            merges.lock().unwrap().as_slice(),
            &[(ArtistId::new(1), ArtistId::new(3), "manual".to_string())]
        );
    }
}
