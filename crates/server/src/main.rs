use anyhow::{Result, bail};
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let env = easy_musiclib_server::env::RuntimeEnv::load_default()?;
    tracing_subscriber::fmt()
        .with_env_filter(
            env.get("RUST_LOG")
                .and_then(|value| EnvFilter::try_new(value).ok())
                .unwrap_or_else(|| EnvFilter::new("easy_musiclib_server=info,tower_http=info")),
        )
        .init();

    let db_path = env.get_or("MUSICLIB_DB", "musiclib.db");
    let static_dir = env.path_or("MUSICLIB_STATIC_DIR", "crates/web/dist");

    let args = std::env::args().collect::<Vec<_>>();
    if args.get(1).map(String::as_str) == Some("import-json") {
        let stat_files = args.iter().skip(2).any(|arg| arg == "--stat-files");
        let json_path = args
            .iter()
            .skip(2)
            .find(|arg| arg.as_str() != "--stat-files")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("library_data.json"));
        let pool = easy_musiclib_server::app::build_pool(&db_path, 1).await?;
        let report = easy_musiclib_server::infra::sqlite::import_old::import_library_data(
            &pool, &json_path, stat_files,
        )
        .await?;
        println!(
            "imported: {} artists, {} albums, {} tracks, {} events",
            report.artists, report.albums, report.tracks, report.events
        );
        return Ok(());
    }

    let bind = env.get_or("MUSICLIB_BIND", "0.0.0.0:5010");
    let addr: SocketAddr = bind.parse()?;

    let tls = easy_musiclib_server::transport::TlsServerConfig::from_paths(
        env.get("MUSICLIB_TLS_CERT").map(PathBuf::from),
        env.get("MUSICLIB_TLS_KEY").map(PathBuf::from),
    )?;
    let mut state = easy_musiclib_server::app::build_state(&db_path, static_dir).await?;
    if tls.is_some() {
        state.transport = easy_musiclib_server::TransportSecurity::encrypted(
            env.get("MUSICLIB_HSTS")
                .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on")),
        );
    }
    let configured_accounts =
        easy_musiclib_server::application::auth::account_count(&state.repositories.auth).await?;
    if configured_accounts > 0 && tls.is_none() {
        bail!(
            "accounts are configured, so MUSICLIB_TLS_CERT and MUSICLIB_TLS_KEY are required before serving"
        );
    }
    let app = easy_musiclib_server::app::router(state);
    tracing::info!(%addr, db_path, "easy_musiclib rust server listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    if let Some(tls) = tls {
        axum::serve(tls.listener(listener), app).await?;
    } else {
        axum::serve(listener, app).await?;
    }
    Ok(())
}
