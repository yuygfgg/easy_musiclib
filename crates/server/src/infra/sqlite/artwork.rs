use crate::application::artwork::ArtworkRepository;
use crate::domain::{ArtworkId, ArtworkSource, MediaFileId};
use anyhow::Result;
use futures::FutureExt;
use futures::future::BoxFuture;
use sqlx::{Row, SqlitePool};

#[derive(Clone)]
pub struct SqliteArtworkRepository {
    pool: SqlitePool,
}

impl SqliteArtworkRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl ArtworkRepository for SqliteArtworkRepository {
    fn source_for_artwork(&self, artwork_id: ArtworkId) -> BoxFuture<'_, Result<ArtworkSource>> {
        async move { source_for_artwork(&self.pool, artwork_id).await }.boxed()
    }

    fn get_artwork_blob<'a>(
        &'a self,
        source_id: ArtworkId,
        variant: &'a str,
    ) -> BoxFuture<'a, Result<Option<(Vec<u8>, String)>>> {
        async move { get_artwork_blob(&self.pool, source_id, variant).await }.boxed()
    }

    fn put_artwork_blob<'a>(
        &'a self,
        source_id: ArtworkId,
        variant: &'a str,
        mime: &'a str,
        width: Option<i64>,
        height: Option<i64>,
        bytes: Vec<u8>,
    ) -> BoxFuture<'a, Result<()>> {
        async move {
            put_artwork_blob(&self.pool, source_id, variant, mime, width, height, bytes).await
        }
        .boxed()
    }
}

async fn source_for_artwork(pool: &SqlitePool, artwork_id: ArtworkId) -> Result<ArtworkSource> {
    let row = sqlx::query(
        "SELECT ars.id, ars.kind, ars.media_file_id, ars.sidecar_path,
                ars.embedded_picture_index, ars.mime, mf.path AS media_path
         FROM artwork_sources ars
         LEFT JOIN media_files mf ON mf.id = ars.media_file_id
         WHERE ars.id = ?",
    )
    .bind(artwork_id.raw())
    .fetch_one(pool)
    .await?;
    let id = ArtworkId::new(row.try_get("id")?);
    let kind: String = row.try_get("kind")?;
    Ok(match kind.as_str() {
        "sidecar" => ArtworkSource::Sidecar {
            id,
            path: row.try_get("sidecar_path")?,
        },
        "embedded" => ArtworkSource::Embedded {
            id,
            media_file_id: row
                .try_get::<Option<i64>, _>("media_file_id")?
                .map(MediaFileId::new),
            media_path: row.try_get("media_path")?,
            picture_index: row
                .try_get::<Option<i64>, _>("embedded_picture_index")?
                .unwrap_or(0),
            mime: row.try_get("mime")?,
        },
        _ => ArtworkSource::Unsupported { id, kind },
    })
}

async fn get_artwork_blob(
    pool: &SqlitePool,
    source_id: ArtworkId,
    variant: &str,
) -> Result<Option<(Vec<u8>, String)>> {
    if let Some(row) =
        sqlx::query("SELECT bytes, mime FROM artwork_blobs WHERE source_id = ? AND variant = ?")
            .bind(source_id.raw())
            .bind(variant)
            .fetch_optional(pool)
            .await?
    {
        Ok(Some((row.try_get("bytes")?, row.try_get("mime")?)))
    } else {
        Ok(None)
    }
}

async fn put_artwork_blob(
    pool: &SqlitePool,
    source_id: ArtworkId,
    variant: &str,
    mime: &str,
    width: Option<i64>,
    height: Option<i64>,
    bytes: Vec<u8>,
) -> Result<()> {
    sqlx::query(
        "INSERT OR REPLACE INTO artwork_blobs
         (source_id, variant, mime, width, height, bytes, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(source_id.raw())
    .bind(variant)
    .bind(mime)
    .bind(width)
    .bind(height)
    .bind(bytes)
    .bind(now_ms())
    .execute(pool)
    .await?;
    Ok(())
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
