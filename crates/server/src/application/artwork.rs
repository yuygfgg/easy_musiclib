use crate::domain::{ArtworkId, ArtworkSource};
use anyhow::Result;
use futures::future::BoxFuture;

pub trait ArtworkRepository: Send + Sync {
    fn source_for_artwork(&self, artwork_id: ArtworkId) -> BoxFuture<'_, Result<ArtworkSource>>;

    fn get_artwork_blob<'a>(
        &'a self,
        source_id: ArtworkId,
        variant: &'a str,
    ) -> BoxFuture<'a, Result<Option<(Vec<u8>, String)>>>;

    fn put_artwork_blob<'a>(
        &'a self,
        source_id: ArtworkId,
        variant: &'a str,
        mime: &'a str,
        width: Option<i64>,
        height: Option<i64>,
        bytes: Vec<u8>,
    ) -> BoxFuture<'a, Result<()>>;
}

pub trait ArtworkSourceReader: Send + Sync {
    fn read_sidecar_artwork<'a>(
        &'a self,
        path: &'a str,
    ) -> BoxFuture<'a, Result<(Vec<u8>, String)>>;

    fn read_embedded_artwork<'a>(
        &'a self,
        path: &'a str,
        picture_index: i64,
    ) -> BoxFuture<'a, Result<(Vec<u8>, String)>>;
}

pub trait ArtworkImageProcessor: Send + Sync {
    fn resize_jpeg(&self, bytes: Vec<u8>, size: u32) -> BoxFuture<'_, Result<Vec<u8>>>;
}

pub async fn source_for_artwork(
    repository: &impl ArtworkRepository,
    artwork_id: ArtworkId,
) -> Result<ArtworkSource> {
    repository.source_for_artwork(artwork_id).await
}

pub async fn get_artwork_blob(
    repository: &impl ArtworkRepository,
    source_id: ArtworkId,
    variant: &str,
) -> Result<Option<(Vec<u8>, String)>> {
    repository.get_artwork_blob(source_id, variant).await
}

pub async fn put_artwork_blob(
    repository: &impl ArtworkRepository,
    source_id: ArtworkId,
    variant: &str,
    mime: &str,
    width: Option<i64>,
    height: Option<i64>,
    bytes: Vec<u8>,
) -> Result<()> {
    repository
        .put_artwork_blob(source_id, variant, mime, width, height, bytes)
        .await
}

pub async fn rendered_artwork(
    repository: &impl ArtworkRepository,
    source_reader: &impl ArtworkSourceReader,
    image_processor: &impl ArtworkImageProcessor,
    artwork_id: ArtworkId,
    size: u32,
) -> Result<(Vec<u8>, String)> {
    let size = size.clamp(32, 2000);
    let variant = format!("size={size}");
    if let Some((bytes, mime)) = repository.get_artwork_blob(artwork_id, &variant).await? {
        return Ok((bytes, mime));
    }

    let source = repository.source_for_artwork(artwork_id).await?;
    let source_id = source.id();
    let (raw, _mime) = match source {
        ArtworkSource::Sidecar { path, .. } => source_reader.read_sidecar_artwork(&path).await?,
        ArtworkSource::Embedded {
            media_path,
            picture_index,
            ..
        } => {
            source_reader
                .read_embedded_artwork(&media_path, picture_index)
                .await?
        }
        ArtworkSource::Unsupported { .. } => {
            return Err(anyhow::anyhow!("unsupported artwork source"));
        }
    };

    let resized = image_processor.resize_jpeg(raw, size).await?;
    repository
        .put_artwork_blob(
            source_id,
            &variant,
            "image/jpeg",
            Some(size as i64),
            None,
            resized.clone(),
        )
        .await
        .ok();
    Ok((resized, "image/jpeg".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::FutureExt;
    use std::sync::{Arc, Mutex};

    struct FakeRepo {
        source: ArtworkSource,
        cached: Option<(Vec<u8>, String)>,
        writes: Arc<Mutex<Vec<(i64, String, String, Vec<u8>)>>>,
    }

    impl ArtworkRepository for FakeRepo {
        fn source_for_artwork(
            &self,
            _artwork_id: ArtworkId,
        ) -> BoxFuture<'_, Result<ArtworkSource>> {
            async move { Ok(self.source.clone()) }.boxed()
        }

        fn get_artwork_blob<'a>(
            &'a self,
            _source_id: ArtworkId,
            _variant: &'a str,
        ) -> BoxFuture<'a, Result<Option<(Vec<u8>, String)>>> {
            async move { Ok(self.cached.clone()) }.boxed()
        }

        fn put_artwork_blob<'a>(
            &'a self,
            source_id: ArtworkId,
            variant: &'a str,
            mime: &'a str,
            _width: Option<i64>,
            _height: Option<i64>,
            bytes: Vec<u8>,
        ) -> BoxFuture<'a, Result<()>> {
            async move {
                self.writes.lock().unwrap().push((
                    source_id.raw(),
                    variant.to_string(),
                    mime.to_string(),
                    bytes,
                ));
                Ok(())
            }
            .boxed()
        }
    }

    struct FakeReader;

    impl ArtworkSourceReader for FakeReader {
        fn read_sidecar_artwork<'a>(
            &'a self,
            _path: &'a str,
        ) -> BoxFuture<'a, Result<(Vec<u8>, String)>> {
            async move { Ok((b"raw".to_vec(), "image/png".to_string())) }.boxed()
        }

        fn read_embedded_artwork<'a>(
            &'a self,
            _path: &'a str,
            _picture_index: i64,
        ) -> BoxFuture<'a, Result<(Vec<u8>, String)>> {
            async move { Ok((b"embedded".to_vec(), "image/png".to_string())) }.boxed()
        }
    }

    struct FakeProcessor;

    impl ArtworkImageProcessor for FakeProcessor {
        fn resize_jpeg(&self, bytes: Vec<u8>, _size: u32) -> BoxFuture<'_, Result<Vec<u8>>> {
            async move {
                let mut out = bytes;
                out.extend_from_slice(b"-resized");
                Ok(out)
            }
            .boxed()
        }
    }

    #[tokio::test]
    async fn returns_cached_artwork_without_reading_source() {
        let repo = FakeRepo {
            source: ArtworkSource::Unsupported {
                id: ArtworkId::new(7),
                kind: "unsupported".to_string(),
            },
            cached: Some((b"cached".to_vec(), "image/jpeg".to_string())),
            writes: Default::default(),
        };

        let (bytes, mime) =
            rendered_artwork(&repo, &FakeReader, &FakeProcessor, ArtworkId::new(7), 256)
                .await
                .unwrap();

        assert_eq!(bytes, b"cached");
        assert_eq!(mime, "image/jpeg");
        assert!(repo.writes.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn reads_resizes_and_caches_sidecar_artwork() {
        let writes = Arc::new(Mutex::new(Vec::new()));
        let repo = FakeRepo {
            source: ArtworkSource::Sidecar {
                id: ArtworkId::new(7),
                path: "cover.png".to_string(),
            },
            cached: None,
            writes: writes.clone(),
        };

        let (bytes, mime) =
            rendered_artwork(&repo, &FakeReader, &FakeProcessor, ArtworkId::new(7), 4)
                .await
                .unwrap();

        assert_eq!(bytes, b"raw-resized");
        assert_eq!(mime, "image/jpeg");
        let writes = writes.lock().unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].0, 7);
        assert_eq!(writes[0].1, "size=32");
    }
}
