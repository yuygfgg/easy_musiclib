use crate::handlers;
use crate::infra::artwork::{FilesystemArtworkSourceReader, ImageArtworkProcessor};
use crate::infra::lyrics::NeteaseLyricsProvider;
use crate::infra::media::{
    FfmpegPlaybackMedia, FilesystemCueSheetReader, FilesystemLibraryFileDiscovery,
    LoftyAudioMetadataReader, MediaArtistNameParser, StaticCueRendererSelector,
};
use crate::infra::sqlite::artists::SqliteArtistRepository;
use crate::infra::sqlite::artwork::SqliteArtworkRepository;
use crate::infra::sqlite::auth::SqliteAuthRepository;
use crate::infra::sqlite::catalog::SqliteCatalogRepository;
use crate::infra::sqlite::lyrics_cache::SqliteLyricsCacheRepository;
use crate::infra::sqlite::maintenance::SqliteMaintenanceRepository;
use crate::infra::sqlite::playback::SqlitePlaybackRepository;
use crate::infra::sqlite::relations::SqliteRelationRepository;
use crate::infra::sqlite::scan_jobs::SqliteScanJobRepository;
use crate::infra::sqlite::scan_library::SqliteScanLibraryRepository;
use crate::infra::sqlite::settings::SqliteSettingsRepository;
use crate::infra::sqlite::track_duration::SqliteTrackDurationRepository;
use crate::{AppRepositories, AppServices, AppState, TransportSecurity};
use anyhow::Result;
use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::routing::{delete, get, post};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use tower_http::compression::predicate::{DefaultPredicate, NotForContentType, Predicate};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

pub const FALLBACK_HTML: &str = include_str!("../../../crates/web/dist/index.html");

pub async fn build_state(db_path: &str, static_dir: PathBuf) -> Result<AppState> {
    build_state_with_max_connections(db_path, static_dir, 4).await
}

pub async fn build_state_with_max_connections(
    db_path: &str,
    static_dir: PathBuf,
    max_connections: u32,
) -> Result<AppState> {
    let pool = build_pool(db_path, max_connections).await?;
    Ok(state_from_pool(pool, static_dir))
}

pub(crate) fn state_from_pool(pool: SqlitePool, static_dir: PathBuf) -> AppState {
    AppState {
        static_dir: Arc::new(static_dir),
        repositories: AppRepositories {
            auth: SqliteAuthRepository::new(pool.clone()),
            catalog: SqliteCatalogRepository::new(pool.clone()),
            settings: SqliteSettingsRepository::new(pool.clone()),
            lyrics_cache: SqliteLyricsCacheRepository::new(pool.clone()),
            scan_jobs: SqliteScanJobRepository::new(pool.clone()),
            scan_library: SqliteScanLibraryRepository::new(pool.clone()),
            artists: SqliteArtistRepository::new(pool.clone()),
            relations: SqliteRelationRepository::new(pool.clone()),
            artwork: SqliteArtworkRepository::new(pool.clone()),
            playback: SqlitePlaybackRepository::new(pool.clone()),
            track_duration: SqliteTrackDurationRepository::new(pool.clone()),
            maintenance: SqliteMaintenanceRepository::new(pool.clone()),
        },
        services: AppServices {
            lyrics_provider: NeteaseLyricsProvider,
            library_file_discovery: FilesystemLibraryFileDiscovery,
            audio_metadata_reader: LoftyAudioMetadataReader,
            cue_sheet_reader: FilesystemCueSheetReader,
            cue_renderer_selector: StaticCueRendererSelector,
            artist_name_parser: MediaArtistNameParser,
            artwork_source_reader: FilesystemArtworkSourceReader,
            artwork_image_processor: ImageArtworkProcessor,
            playback_media: FfmpegPlaybackMedia,
        },
        transport: TransportSecurity::plaintext(),
    }
}

pub async fn build_pool(db_path: &str, max_connections: u32) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{db_path}"))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect_with(options)
        .await?;
    crate::infra::sqlite::schema::init_db(&pool).await?;
    Ok(pool)
}

pub fn router(state: AppState) -> Router {
    let static_dir = (*state.static_dir).clone();
    let auth_state = state.clone();
    Router::new()
        .route("/", get(handlers::index))
        .route("/liked", get(handlers::index))
        .route("/search", get(handlers::index))
        .route("/albums/{id}", get(handlers::index))
        .route("/artists/{id}", get(handlers::index))
        .route("/events/{id}", get(handlers::index))
        .route("/relations", get(handlers::index))
        .route("/settings", get(handlers::index))
        .route("/api/tracks", get(handlers::list_tracks))
        .route(
            "/api/tracks/{id}",
            get(handlers::get_track).patch(handlers::patch_track),
        )
        .route("/api/tracks/{id}/stream", get(handlers::stream_track))
        .route("/api/tracks/{id}/raw", get(handlers::raw_track))
        .route(
            "/api/tracks/{id}/hls/{file}",
            get(handlers::stream_track_hls_file),
        )
        .route("/api/tracks/{id}/download", get(handlers::download_track))
        .route("/api/albums", get(handlers::list_albums))
        .route(
            "/api/albums/{id}",
            get(handlers::get_album).patch(handlers::patch_album),
        )
        .route(
            "/api/artists",
            get(handlers::list_artists).post(handlers::create_artist),
        )
        .route(
            "/api/artists/{id}",
            get(handlers::get_artist).patch(handlers::patch_artist),
        )
        .route("/api/artists/{id}/aliases", post(handlers::add_alias))
        .route("/api/artists/merge", post(handlers::merge_artists))
        .route("/api/artists/auto-merge", post(handlers::auto_merge))
        .route(
            "/api/artists/alias-csv-import",
            post(handlers::alias_csv_import),
        )
        .route("/api/events", get(handlers::list_events))
        .route(
            "/api/events/{id}",
            get(handlers::get_event).patch(handlers::patch_event),
        )
        .route("/api/search", get(handlers::search))
        .route("/api/relations", get(handlers::relations))
        .route("/api/artwork/{id}", get(handlers::artwork))
        .route("/api/lyrics/search", get(handlers::lyrics_search))
        .route("/api/auth/status", get(handlers::auth_status))
        .route("/api/auth/login", post(handlers::login))
        .route("/api/auth/logout", post(handlers::logout))
        .route(
            "/api/settings",
            get(handlers::get_settings).patch(handlers::patch_settings),
        )
        .route(
            "/api/settings/accounts",
            get(handlers::list_accounts).post(handlers::create_account),
        )
        .route(
            "/api/settings/accounts/{username}",
            delete(handlers::delete_account).patch(handlers::update_account_password),
        )
        .route("/api/cache/hls/clear", post(handlers::clear_hls_cache))
        .route("/api/scan-jobs", post(handlers::create_scan_job))
        .route("/api/scan-jobs/{id}", get(handlers::get_scan_job))
        .route(
            "/api/scan-jobs/{id}/cancel",
            post(handlers::cancel_scan_job),
        )
        .route("/api/database/vacuum", post(handlers::vacuum))
        .fallback_service(ServeDir::new(static_dir).append_index_html_on_directories(true))
        .layer(
            CompressionLayer::new()
                .compress_when(DefaultPredicate::new().and(NotForContentType::const_new("audio/"))),
        )
        .layer(from_fn_with_state(
            auth_state,
            crate::http::auth::require_auth,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
