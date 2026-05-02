use crate::domain::{AppSettings, UpdateAppSettings};
use anyhow::Result;
use futures::future::BoxFuture;

pub trait SettingsRepository: Send + Sync {
    fn get_settings(&self) -> BoxFuture<'_, Result<AppSettings>>;

    fn update_settings(&self, req: UpdateAppSettings) -> BoxFuture<'_, Result<AppSettings>>;
}

pub async fn get_settings(repository: &impl SettingsRepository) -> Result<AppSettings> {
    repository.get_settings().await
}

pub async fn update_settings(
    repository: &impl SettingsRepository,
    req: UpdateAppSettings,
) -> Result<AppSettings> {
    repository.update_settings(req.normalized()).await
}
