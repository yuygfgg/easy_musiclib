use crate::domain::{ScanJob, ScanJobId, ScanJobState};
use anyhow::Result;
use futures::future::BoxFuture;

pub trait ScanJobRepository: Send + Sync {
    fn insert_or_update_scan_job<'a>(
        &'a self,
        roots: &'a [String],
    ) -> BoxFuture<'a, Result<ScanJob>>;

    fn scan_job(&self, id: ScanJobId) -> BoxFuture<'_, Result<ScanJob>>;

    fn update_scan_job_counts<'a>(
        &'a self,
        id: ScanJobId,
        state: ScanJobState,
        total: Option<i64>,
        scanned: Option<i64>,
        error: Option<&'a str>,
        finished: bool,
    ) -> BoxFuture<'a, Result<()>>;
}

pub async fn insert_or_update_scan_job(
    repository: &impl ScanJobRepository,
    roots: &[String],
) -> Result<ScanJob> {
    repository.insert_or_update_scan_job(roots).await
}

pub async fn scan_job(repository: &impl ScanJobRepository, id: ScanJobId) -> Result<ScanJob> {
    repository.scan_job(id).await
}

pub async fn update_scan_job_counts(
    repository: &impl ScanJobRepository,
    id: ScanJobId,
    state: ScanJobState,
    total: Option<i64>,
    scanned: Option<i64>,
    error: Option<&str>,
    finished: bool,
) -> Result<()> {
    repository
        .update_scan_job_counts(id, state, total, scanned, error, finished)
        .await
}
