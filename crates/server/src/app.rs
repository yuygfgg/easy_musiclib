use crate::handlers;
use crate::{AppState, schema};
use anyhow::Result;
use axum::Router;
use axum::routing::{get, post};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use tower_http::compression::predicate::{DefaultPredicate, NotForContentType, Predicate};
use tower_http::cors::CorsLayer;
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
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{db_path}"))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect_with(options)
        .await?;
    schema::init_db(&pool).await?;
    Ok(AppState {
        pool,
        static_dir: Arc::new(static_dir),
    })
}

pub fn router(state: AppState) -> Router {
    let static_dir = (*state.static_dir).clone();
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
        .route(
            "/api/settings",
            get(handlers::get_settings).patch(handlers::patch_settings),
        )
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
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
