use crate::domain::{LyricsCandidate, TrackId};
use anyhow::Result;
use futures::future::BoxFuture;

pub trait LyricsCacheRepository: Send + Sync {
    fn cached_lyrics<'a>(
        &'a self,
        track_id: Option<TrackId>,
        title: &'a str,
        artist: &'a str,
    ) -> BoxFuture<'a, Result<Vec<LyricsCandidate>>>;

    fn cache_lyrics<'a>(
        &'a self,
        track_id: Option<TrackId>,
        candidate: &'a LyricsCandidate,
    ) -> BoxFuture<'a, Result<()>>;
}

pub trait LyricsProvider: Send + Sync {
    fn search_lyrics<'a>(
        &'a self,
        title: &'a str,
        artist: &'a str,
        album: Option<&'a str>,
        duration_ms: Option<i64>,
    ) -> BoxFuture<'a, Result<Vec<LyricsCandidate>>>;
}

pub struct LyricsSearchInput {
    pub track_id: Option<TrackId>,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_ms: Option<i64>,
}

pub async fn cached_lyrics(
    repository: &impl LyricsCacheRepository,
    track_id: Option<TrackId>,
    title: &str,
    artist: &str,
) -> Result<Vec<LyricsCandidate>> {
    repository.cached_lyrics(track_id, title, artist).await
}

pub async fn cache_lyrics(
    repository: &impl LyricsCacheRepository,
    track_id: Option<TrackId>,
    candidate: &LyricsCandidate,
) -> Result<()> {
    repository.cache_lyrics(track_id, candidate).await
}

pub async fn search_lyrics(
    cache: &impl LyricsCacheRepository,
    provider: &impl LyricsProvider,
    input: LyricsSearchInput,
) -> Result<Vec<LyricsCandidate>> {
    let cached = cache
        .cached_lyrics(input.track_id, &input.title, &input.artist)
        .await?;
    if !cached.is_empty() {
        return Ok(cached);
    }

    let results = provider
        .search_lyrics(
            &input.title,
            &input.artist,
            input.album.as_deref(),
            input.duration_ms,
        )
        .await?;
    for item in &results {
        cache.cache_lyrics(input.track_id, item).await.ok();
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use futures::FutureExt;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct FakeCache {
        cached: Vec<LyricsCandidate>,
        writes: Arc<Mutex<Vec<LyricsCandidate>>>,
    }

    impl LyricsCacheRepository for FakeCache {
        fn cached_lyrics<'a>(
            &'a self,
            _track_id: Option<TrackId>,
            _title: &'a str,
            _artist: &'a str,
        ) -> BoxFuture<'a, Result<Vec<LyricsCandidate>>> {
            async move { Ok(self.cached.clone()) }.boxed()
        }

        fn cache_lyrics<'a>(
            &'a self,
            _track_id: Option<TrackId>,
            candidate: &'a LyricsCandidate,
        ) -> BoxFuture<'a, Result<()>> {
            async move {
                self.writes.lock().unwrap().push(candidate.clone());
                Ok(())
            }
            .boxed()
        }
    }

    struct FakeProvider {
        results: Vec<LyricsCandidate>,
    }

    impl LyricsProvider for FakeProvider {
        fn search_lyrics<'a>(
            &'a self,
            _title: &'a str,
            _artist: &'a str,
            _album: Option<&'a str>,
            _duration_ms: Option<i64>,
        ) -> BoxFuture<'a, Result<Vec<LyricsCandidate>>> {
            async move { Ok(self.results.clone()) }.boxed()
        }
    }

    #[tokio::test]
    async fn returns_cached_lyrics_without_provider_lookup() {
        let cached = lyric("cached");
        let cache = FakeCache {
            cached: vec![cached.clone()],
            writes: Default::default(),
        };
        let provider = FakeProvider {
            results: vec![lyric("provider")],
        };

        let results = search_lyrics(&cache, &provider, input()).await.unwrap();

        assert_eq!(results, vec![cached]);
        assert!(cache.writes.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn caches_provider_results_on_miss() {
        let found = lyric("provider");
        let writes = Arc::new(Mutex::new(Vec::new()));
        let cache = FakeCache {
            cached: Vec::new(),
            writes: writes.clone(),
        };
        let provider = FakeProvider {
            results: vec![found.clone()],
        };

        let results = search_lyrics(&cache, &provider, input()).await.unwrap();

        assert_eq!(results, vec![found.clone()]);
        assert_eq!(writes.lock().unwrap().as_slice(), &[found]);
    }

    fn input() -> LyricsSearchInput {
        LyricsSearchInput {
            track_id: Some(TrackId::new(1)),
            title: "Title".to_string(),
            artist: "Artist".to_string(),
            album: None,
            duration_ms: None,
        }
    }

    fn lyric(title: &str) -> LyricsCandidate {
        LyricsCandidate {
            title: title.to_string(),
            artist: "Artist".to_string(),
            album: None,
            duration_ms: None,
            lyrics: "[00:00]line".to_string(),
            score: 1.0,
            provider: "fake".to_string(),
        }
    }
}
