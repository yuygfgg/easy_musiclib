use anyhow::Result;
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("easy_musiclib_server=info,tower_http=info")),
        )
        .init();

    let db_path = std::env::var("MUSICLIB_DB").unwrap_or_else(|_| "musiclib.db".to_string());
    let static_dir = std::env::var("MUSICLIB_STATIC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("crates/web/dist"));

    let args = std::env::args().collect::<Vec<_>>();
    if args.get(1).map(String::as_str) == Some("import-json") {
        let stat_files = args.iter().skip(2).any(|arg| arg == "--stat-files");
        let json_path = args
            .iter()
            .skip(2)
            .find(|arg| arg.as_str() != "--stat-files")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("library_data.json"));
        let state =
            easy_musiclib_server::app::build_state_with_max_connections(&db_path, static_dir, 1)
                .await?;
        let report = easy_musiclib_server::import_old::import_library_data(
            &state.pool,
            &json_path,
            stat_files,
        )
        .await?;
        println!(
            "imported: {} artists, {} albums, {} tracks, {} events",
            report.artists, report.albums, report.tracks, report.events
        );
        return Ok(());
    }

    let bind = std::env::var("MUSICLIB_BIND").unwrap_or_else(|_| "0.0.0.0:5010".to_string());
    let addr: SocketAddr = bind.parse()?;

    let state = easy_musiclib_server::app::build_state(&db_path, static_dir).await?;
    let app = easy_musiclib_server::app::router(state);
    tracing::info!(%addr, db_path, "easy_musiclib rust server listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
