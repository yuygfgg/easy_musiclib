use crate::application::maintenance::DatabaseMaintenanceRepository;
use anyhow::Result;
use futures::FutureExt;
use futures::future::BoxFuture;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct SqliteMaintenanceRepository {
    pool: SqlitePool,
}

impl SqliteMaintenanceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl DatabaseMaintenanceRepository for SqliteMaintenanceRepository {
    fn vacuum(&self) -> BoxFuture<'_, Result<()>> {
        async move {
            sqlx::query("VACUUM").execute(&self.pool).await?;
            Ok(())
        }
        .boxed()
    }
}
