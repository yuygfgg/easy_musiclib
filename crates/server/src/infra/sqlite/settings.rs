use crate::application::settings::SettingsRepository;
use crate::domain::{AppSettings, BrowserPlaybackFormat, UpdateAppSettings};
use anyhow::Result;
use futures::FutureExt;
use futures::future::BoxFuture;
use sqlx::{Row, SqlitePool};

const BROWSER_PLAYBACK_FORMAT_SETTING: &str = "browser_playback_format";

#[derive(Clone)]
pub struct SqliteSettingsRepository {
    pool: SqlitePool,
}

impl SqliteSettingsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl SettingsRepository for SqliteSettingsRepository {
    fn get_settings(&self) -> BoxFuture<'_, Result<AppSettings>> {
        async move { app_settings(&self.pool).await }.boxed()
    }

    fn update_settings(&self, req: UpdateAppSettings) -> BoxFuture<'_, Result<AppSettings>> {
        async move { update_app_settings(&self.pool, req).await }.boxed()
    }
}

async fn app_settings(pool: &SqlitePool) -> Result<AppSettings> {
    Ok(AppSettings {
        browser_playback_format: setting_json::<StoredBrowserPlaybackFormat>(
            pool,
            BROWSER_PLAYBACK_FORMAT_SETTING,
        )
        .await?
        .map(Into::into)
        .unwrap_or_default(),
    })
}

async fn update_app_settings(pool: &SqlitePool, req: UpdateAppSettings) -> Result<AppSettings> {
    let format: StoredBrowserPlaybackFormat = req.browser_playback_format.into();
    put_setting_json(pool, BROWSER_PLAYBACK_FORMAT_SETTING, &format).await?;
    app_settings(pool).await
}

async fn setting_json<T>(pool: &SqlitePool, key: &str) -> Result<Option<T>>
where
    T: serde::de::DeserializeOwned,
{
    let Some(row) = sqlx::query("SELECT value_json FROM app_settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(None);
    };
    let raw: String = row.try_get("value_json")?;
    Ok(Some(serde_json::from_str(&raw)?))
}

async fn put_setting_json<T>(pool: &SqlitePool, key: &str, value: &T) -> Result<()>
where
    T: serde::Serialize,
{
    sqlx::query(
        "INSERT INTO app_settings (key, value_json, updated_at)
         VALUES (?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
    )
    .bind(key)
    .bind(serde_json::to_string(value)?)
    .bind(now_ms())
    .execute(pool)
    .await?;
    Ok(())
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
enum StoredBrowserPlaybackFormat {
    #[serde(rename = "opus_256k")]
    Opus256k,
    #[serde(rename = "flac_48k")]
    Flac48k,
}

impl From<StoredBrowserPlaybackFormat> for BrowserPlaybackFormat {
    fn from(value: StoredBrowserPlaybackFormat) -> Self {
        match value {
            StoredBrowserPlaybackFormat::Opus256k => Self::Opus256k,
            StoredBrowserPlaybackFormat::Flac48k => Self::Flac48k,
        }
    }
}

impl From<BrowserPlaybackFormat> for StoredBrowserPlaybackFormat {
    fn from(value: BrowserPlaybackFormat) -> Self {
        match value {
            BrowserPlaybackFormat::Opus256k => Self::Opus256k,
            BrowserPlaybackFormat::Flac48k => Self::Flac48k,
        }
    }
}
