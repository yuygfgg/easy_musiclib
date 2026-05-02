use crate::application::catalog::CatalogRepository;
use crate::application::scan::AudioMetadataReader;
use crate::domain::{TrackDetail, TrackId};
use anyhow::Result;
use futures::future::BoxFuture;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct TrackDurationSource {
    pub track_id: TrackId,
    pub track_duration_ms: Option<i64>,
    pub kind: Option<String>,
    pub media_file_id: Option<i64>,
    pub path: Option<String>,
    pub media_duration_ms: Option<i64>,
    pub sample_rate: Option<i64>,
    pub start_sample: Option<i64>,
    pub end_sample: Option<i64>,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
}

pub trait TrackDurationRepository: Send + Sync {
    fn track_duration_source(
        &self,
        id: TrackId,
    ) -> BoxFuture<'_, Result<Option<TrackDurationSource>>>;

    fn persist_track_duration_ms(
        &self,
        source: TrackDurationSource,
        duration_ms: i64,
    ) -> BoxFuture<'_, Result<()>>;
}

pub async fn fetch_track_detail_with_duration(
    catalog: &impl CatalogRepository,
    durations: &impl TrackDurationRepository,
    metadata_reader: &impl AudioMetadataReader,
    id: TrackId,
) -> Result<TrackDetail> {
    ensure_track_duration_ms(durations, metadata_reader, id).await?;
    catalog.fetch_track_detail(id).await
}

pub async fn ensure_track_duration_ms(
    repository: &impl TrackDurationRepository,
    metadata_reader: &impl AudioMetadataReader,
    id: TrackId,
) -> Result<()> {
    let Some(source) = repository.track_duration_source(id).await? else {
        return Ok(());
    };
    if source.track_duration_ms.is_some() {
        return Ok(());
    }

    match infer_track_duration_ms(metadata_reader, &source).await {
        Ok(Some(duration_ms)) => {
            repository
                .persist_track_duration_ms(source, duration_ms)
                .await?;
        }
        Ok(None) => {}
        Err(err) => {
            tracing::warn!(
                track_id = id.raw(),
                path = %source.path.as_deref().unwrap_or(""),
                error = %err,
                "failed to infer track duration"
            );
        }
    }
    Ok(())
}

async fn infer_track_duration_ms(
    metadata_reader: &impl AudioMetadataReader,
    source: &TrackDurationSource,
) -> Result<Option<i64>> {
    if let (Some(start_ms), Some(end_ms)) = (source.start_ms, source.end_ms) {
        return Ok(positive_duration(end_ms.saturating_sub(start_ms)));
    }
    if let (Some(sample_rate), Some(start_sample), Some(end_sample)) =
        (source.sample_rate, source.start_sample, source.end_sample)
    {
        if sample_rate > 0 {
            return Ok(positive_duration(
                end_sample.saturating_sub(start_sample).saturating_mul(1000) / sample_rate,
            ));
        }
    }

    let file_duration_ms = match source.media_duration_ms {
        Some(duration_ms) => Some(duration_ms),
        None => read_source_duration_ms(metadata_reader, source).await?,
    };
    let Some(file_duration_ms) = file_duration_ms else {
        return Ok(None);
    };
    if source.kind.as_deref() == Some("cue") {
        let start_ms = source
            .start_ms
            .or_else(|| cue_start_ms_from_samples(source))
            .unwrap_or(0);
        return Ok(positive_duration(file_duration_ms.saturating_sub(start_ms)));
    }
    Ok(positive_duration(file_duration_ms))
}

async fn read_source_duration_ms(
    metadata_reader: &impl AudioMetadataReader,
    source: &TrackDurationSource,
) -> Result<Option<i64>> {
    let Some(path) = source.path.as_ref() else {
        return Ok(None);
    };
    let path = PathBuf::from(path);
    let tags = metadata_reader.read_audio_metadata(&path, &[]).await?;
    Ok(tags.duration_ms)
}

fn cue_start_ms_from_samples(source: &TrackDurationSource) -> Option<i64> {
    let sample_rate = source.sample_rate?;
    let start_sample = source.start_sample?;
    (sample_rate > 0).then_some(start_sample.saturating_mul(1000) / sample_rate)
}

fn positive_duration(duration_ms: i64) -> Option<i64> {
    (duration_ms > 0).then_some(duration_ms)
}
