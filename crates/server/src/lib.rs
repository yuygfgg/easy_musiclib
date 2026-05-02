pub mod app;
pub mod application;
pub mod contracts;
pub mod domain;
pub mod handlers;
pub mod http;
pub mod infra;
pub mod services;

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use easy_musiclib_shared::ApiError;
use infra::artwork::{FilesystemArtworkSourceReader, ImageArtworkProcessor};
use infra::lyrics::NeteaseLyricsProvider;
use infra::media::{
    FfmpegPlaybackMedia, FilesystemCueSheetReader, FilesystemLibraryFileDiscovery,
    LoftyAudioMetadataReader, MediaArtistNameParser, StaticCueRendererSelector,
};
use infra::sqlite::artists::SqliteArtistRepository;
use infra::sqlite::artwork::SqliteArtworkRepository;
use infra::sqlite::catalog::SqliteCatalogRepository;
use infra::sqlite::lyrics_cache::SqliteLyricsCacheRepository;
use infra::sqlite::maintenance::SqliteMaintenanceRepository;
use infra::sqlite::playback::SqlitePlaybackRepository;
use infra::sqlite::relations::SqliteRelationRepository;
use infra::sqlite::scan_jobs::SqliteScanJobRepository;
use infra::sqlite::scan_library::SqliteScanLibraryRepository;
use infra::sqlite::settings::SqliteSettingsRepository;
use infra::sqlite::track_duration::SqliteTrackDurationRepository;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub static_dir: Arc<PathBuf>,
    pub repositories: AppRepositories,
    pub services: AppServices,
}

#[derive(Clone)]
pub struct AppRepositories {
    pub catalog: SqliteCatalogRepository,
    pub settings: SqliteSettingsRepository,
    pub lyrics_cache: SqliteLyricsCacheRepository,
    pub scan_jobs: SqliteScanJobRepository,
    pub scan_library: SqliteScanLibraryRepository,
    pub artists: SqliteArtistRepository,
    pub relations: SqliteRelationRepository,
    pub artwork: SqliteArtworkRepository,
    pub playback: SqlitePlaybackRepository,
    pub track_duration: SqliteTrackDurationRepository,
    pub maintenance: SqliteMaintenanceRepository,
}

#[derive(Clone)]
pub struct AppServices {
    pub lyrics_provider: NeteaseLyricsProvider,
    pub library_file_discovery: FilesystemLibraryFileDiscovery,
    pub audio_metadata_reader: LoftyAudioMetadataReader,
    pub cue_sheet_reader: FilesystemCueSheetReader,
    pub cue_renderer_selector: StaticCueRendererSelector,
    pub artist_name_parser: MediaArtistNameParser,
    pub artwork_source_reader: FilesystemArtworkSourceReader,
    pub artwork_image_processor: ImageArtworkProcessor,
    pub playback_media: FfmpegPlaybackMedia,
}

#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(value: anyhow::Error) -> Self {
        tracing::error!(error = %value, "request failed");
        Self::internal(value.to_string())
    }
}

impl From<sqlx::Error> for AppError {
    fn from(value: sqlx::Error) -> Self {
        tracing::error!(error = %value, "database request failed");
        Self::internal(value.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        tracing::error!(error = %value, "io request failed");
        Self::internal(value.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ApiError {
                message: self.message,
            }),
        )
            .into_response()
    }
}

pub type ApiResult<T> = Result<T, AppError>;
