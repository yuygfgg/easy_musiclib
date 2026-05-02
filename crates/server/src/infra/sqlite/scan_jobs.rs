use crate::application::scan_jobs::ScanJobRepository;
use crate::domain::{ScanJob, ScanJobId, ScanJobState};
use anyhow::Result;
use futures::FutureExt;
use futures::future::BoxFuture;
use sqlx::{Row, SqlitePool};

#[derive(Clone)]
pub struct SqliteScanJobRepository {
    pool: SqlitePool,
}

impl SqliteScanJobRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl ScanJobRepository for SqliteScanJobRepository {
    fn insert_or_update_scan_job<'a>(
        &'a self,
        roots: &'a [String],
    ) -> BoxFuture<'a, Result<ScanJob>> {
        async move { insert_or_update_scan_job(&self.pool, roots).await }.boxed()
    }

    fn scan_job(&self, id: ScanJobId) -> BoxFuture<'_, Result<ScanJob>> {
        async move { scan_job(&self.pool, id).await }.boxed()
    }

    fn update_scan_job_counts<'a>(
        &'a self,
        id: ScanJobId,
        state: ScanJobState,
        total: Option<i64>,
        scanned: Option<i64>,
        error: Option<&'a str>,
        finished: bool,
    ) -> BoxFuture<'a, Result<()>> {
        async move {
            update_scan_job_counts(&self.pool, id, state, total, scanned, error, finished).await
        }
        .boxed()
    }
}

async fn insert_or_update_scan_job(pool: &SqlitePool, roots: &[String]) -> Result<ScanJob> {
    let root_paths = serde_json::to_string(roots)?;
    let res = sqlx::query(
        "INSERT INTO scan_jobs (status, root_paths, started_at) VALUES ('queued', ?, ?)",
    )
    .bind(root_paths)
    .bind(now_ms())
    .execute(pool)
    .await?;
    scan_job(pool, ScanJobId::new(res.last_insert_rowid())).await
}

async fn scan_job(pool: &SqlitePool, id: ScanJobId) -> Result<ScanJob> {
    let row = sqlx::query("SELECT * FROM scan_jobs WHERE id = ?")
        .bind(id.raw())
        .fetch_one(pool)
        .await?;
    scan_job_from_row(row)
}

fn scan_job_from_row(row: sqlx::sqlite::SqliteRow) -> Result<ScanJob> {
    let root_paths_json: String = row.try_get("root_paths")?;
    let status: String = row.try_get("status")?;
    Ok(ScanJob {
        id: ScanJobId::new(row.try_get("id")?),
        state: ScanJobState::from(status.as_str()),
        root_paths: serde_json::from_str(&root_paths_json).unwrap_or_default(),
        total_files: row.try_get("total_files")?,
        scanned_files: row.try_get("scanned_files")?,
        started_at: row.try_get("started_at")?,
        finished_at: row.try_get("finished_at")?,
        error: row.try_get("error")?,
    })
}

async fn update_scan_job_counts(
    pool: &SqlitePool,
    id: ScanJobId,
    state: ScanJobState,
    total: Option<i64>,
    scanned: Option<i64>,
    error: Option<&str>,
    finished: bool,
) -> Result<()> {
    sqlx::query(
        "UPDATE scan_jobs
         SET status = ?,
             total_files = COALESCE(?, total_files),
             scanned_files = COALESCE(?, scanned_files),
             error = COALESCE(?, error),
             finished_at = CASE WHEN ? THEN ? ELSE finished_at END
         WHERE id = ?",
    )
    .bind(state.as_str())
    .bind(total)
    .bind(scanned)
    .bind(error)
    .bind(finished)
    .bind(now_ms())
    .bind(id.raw())
    .execute(pool)
    .await?;
    Ok(())
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
