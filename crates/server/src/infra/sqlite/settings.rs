use crate::application::settings::SettingsRepository;
use crate::domain::{
    AppSettings, BrowserPlaybackFormat, BrowserPlaybackSettings, UpdateAppSettings,
};
use anyhow::Result;
use easy_musiclib_shared::{
    DEFAULT_BROWSER_PLAYBACK_FLAC_SAMPLE_RATE, DEFAULT_BROWSER_PLAYBACK_OPUS_BITRATE,
};
use futures::FutureExt;
use futures::future::BoxFuture;
use sqlx::{Row, SqlitePool};

const BROWSER_PLAYBACK_SETTING: &str = "browser_playback";

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
        browser_playback: setting_json::<StoredBrowserPlaybackSettings>(
            pool,
            BROWSER_PLAYBACK_SETTING,
        )
        .await?
        .map(Into::into)
        .unwrap_or_default(),
    }
    .normalized())
}

async fn update_app_settings(pool: &SqlitePool, req: UpdateAppSettings) -> Result<AppSettings> {
    let req = req.normalized();
    let playback: StoredBrowserPlaybackSettings = req.browser_playback.into();
    put_setting_json(pool, BROWSER_PLAYBACK_SETTING, &playback).await?;
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
    #[serde(rename = "opus")]
    Opus,
    #[serde(rename = "flac")]
    Flac,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
struct StoredBrowserPlaybackSettings {
    #[serde(default)]
    format: StoredBrowserPlaybackFormat,
    #[serde(default = "default_opus_bitrate")]
    opus_bitrate: i64,
    #[serde(default = "default_flac_sample_rate")]
    flac_sample_rate: i64,
}

impl Default for StoredBrowserPlaybackSettings {
    fn default() -> Self {
        Self {
            format: StoredBrowserPlaybackFormat::default(),
            opus_bitrate: DEFAULT_BROWSER_PLAYBACK_OPUS_BITRATE,
            flac_sample_rate: DEFAULT_BROWSER_PLAYBACK_FLAC_SAMPLE_RATE,
        }
    }
}

fn default_opus_bitrate() -> i64 {
    DEFAULT_BROWSER_PLAYBACK_OPUS_BITRATE
}

fn default_flac_sample_rate() -> i64 {
    DEFAULT_BROWSER_PLAYBACK_FLAC_SAMPLE_RATE
}

impl Default for StoredBrowserPlaybackFormat {
    fn default() -> Self {
        Self::Opus
    }
}

impl From<StoredBrowserPlaybackSettings> for BrowserPlaybackSettings {
    fn from(value: StoredBrowserPlaybackSettings) -> Self {
        Self {
            format: value.format.into(),
            opus_bitrate: value.opus_bitrate,
            flac_sample_rate: value.flac_sample_rate,
        }
    }
}

impl From<BrowserPlaybackSettings> for StoredBrowserPlaybackSettings {
    fn from(value: BrowserPlaybackSettings) -> Self {
        Self {
            format: value.format.into(),
            opus_bitrate: value.opus_bitrate,
            flac_sample_rate: value.flac_sample_rate,
        }
    }
}

impl From<StoredBrowserPlaybackFormat> for BrowserPlaybackFormat {
    fn from(value: StoredBrowserPlaybackFormat) -> Self {
        match value {
            StoredBrowserPlaybackFormat::Opus => Self::Opus,
            StoredBrowserPlaybackFormat::Flac => Self::Flac,
        }
    }
}

impl From<BrowserPlaybackFormat> for StoredBrowserPlaybackFormat {
    fn from(value: BrowserPlaybackFormat) -> Self {
        match value {
            BrowserPlaybackFormat::Opus => Self::Opus,
            BrowserPlaybackFormat::Flac => Self::Flac,
        }
    }
}
