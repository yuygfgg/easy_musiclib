use anyhow::Result;
use futures::future::BoxFuture;

pub trait DatabaseMaintenanceRepository: Send + Sync {
    fn vacuum(&self) -> BoxFuture<'_, Result<()>>;
}

pub async fn vacuum(repository: &impl DatabaseMaintenanceRepository) -> Result<()> {
    repository.vacuum().await
}
