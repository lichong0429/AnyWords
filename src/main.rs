// DocSeek - Web-based File Full-Text Search Engine
// Main entry point

mod api;
mod config;
mod index;
mod parser;
mod watcher;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;
use tower_http::cors::{CorsLayer, Any};
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;
use index::engine::SearchEngine;
use parser::tika::TikaParser;
use api::router::create_router;

/// Application state shared across all handlers
pub struct AppState {
    pub engine: SearchEngine,
    pub config: Config,
    pub tika: Option<TikaParser>,
    /// Indexing progress: (current, total, message)
    pub index_progress: TokioRwLock<(usize, usize, String)>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = Config::load(None)?;

    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("docseek={}", config.logging.level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("DocSeek v{} starting up...", env!("CARGO_PKG_VERSION"));
    tracing::info!("Config loaded: port={}, index_dir={}",
        config.server.port,
        config.index.dir.display()
    );

    // Create index directory if needed
    std::fs::create_dir_all(&config.index.dir)?;

    // Initialize Tika parser if configured
    let tika = if config.parser.tika_server_url.is_some() || config.parser.tika_jar_path.is_some() {
        let tika_config = parser::tika::TikaConfig {
            server_url: config.parser.tika_server_url.clone(),
            jar_path: config.parser.tika_jar_path.clone(),
            fallback: config.parser.fallback_basic,
        };
        let parser = TikaParser::new(tika_config);
        if parser.is_available() {
            tracing::info!("Tika parser enabled for enhanced document parsing");
            Some(parser)
        } else {
            tracing::info!("Tika configured but not available - using basic parser");
            None
        }
    } else {
        tracing::info!("Tika not configured - using basic parser only");
        None
    };

    // Initialize search engine
    let engine = SearchEngine::open_or_create(&config.index.dir)?;
    let stats = engine.stats().unwrap_or_default();
    tracing::info!(
        "Search engine ready: {} documents indexed, {} MB",
        stats.total_docs,
        stats.index_size_bytes / 1024 / 1024
    );

    let state = Arc::new(AppState {
        engine,
        config: config.clone(),
        tika,
        index_progress: TokioRwLock::new((0, 0, String::new())),
    });

    // Start file watcher in background
    if config.watcher.enabled && !config.watcher.watch_dirs.is_empty() {
        let watch_state = state.clone();
        let watch_dirs: Vec<std::path::PathBuf> = config.watcher.watch_dirs
            .iter()
            .map(std::path::PathBuf::from)
            .collect();

        watcher::monitor::start_watcher(watch_state, watch_dirs.clone())?;
        tracing::info!(
            "File watcher started for {} director{}",
            watch_dirs.len(),
            if watch_dirs.len() > 1 { "ies" } else { "y" }
        );

        // Start periodic full scan if configured
        if config.watcher.full_scan_interval_secs > 0 {
            let scan_state = state.clone();
            let interval = config.watcher.full_scan_interval_secs;
            tokio::spawn(async move {
                periodic_full_scan(scan_state, interval).await;
            });
            tracing::info!("Periodic full scan scheduled every {} seconds", interval);
        }
    }

    // Auto-index watched directories on startup
    if !config.watcher.watch_dirs.is_empty() {
        let init_state = state.clone();
        let init_dirs = config.watcher.watch_dirs.clone();
        tokio::spawn(async move {
            scan_directories_on_startup(init_state, init_dirs).await;
        });
    }

    // Build router
    let app = create_router(state)
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .fallback_service(ServeDir::new("frontend/dist"));

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], config.server.port));
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    tracing::info!("  DocSeek is running at: http://{}", addr);
    tracing::info!("  Open your browser to start searching!");
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Scan configured directories on startup for initial indexing
async fn scan_directories_on_startup(
    state: Arc<AppState>,
    dirs: Vec<String>,
) {
    tracing::info!("Starting initial index scan...");

    let mut total_files = 0usize;
    let mut indexed = 0usize;
    let mut errors = 0usize;

    // Count total files first
    for dir in &dirs {
        if let Ok(count) = api::index_api::count_files_in_dir(dir, &state.config) {
            total_files += count;
        }
    }

    {
        let mut progress = state.index_progress.write().await;
        *progress = (0, total_files, "Starting initial scan...".to_string());
    }

    // Scan and index
    for dir in &dirs {
        match api::index_api::index_directory(dir, &state, &|_current, _total, _msg| {
            // This closure is called for progress updates
            // In a real implementation, we'd update state.index_progress
        }) {
            Ok((idx, err)) => {
                indexed += idx;
                errors += err;
            }
            Err(e) => {
                tracing::error!("Failed to scan {}: {}", dir, e);
            }
        }
    }

    {
        let mut progress = state.index_progress.write().await;
        *progress = (indexed, indexed, "Initial scan complete".to_string());
    }

    tracing::info!(
        "Initial scan complete: {} indexed, {} errors",
        indexed, errors
    );
}

/// Periodic full scan of watched directories
async fn periodic_full_scan(state: Arc<AppState>, interval_secs: u64) {
    let mut interval = tokio::time::interval(
        std::time::Duration::from_secs(interval_secs)
    );

    loop {
        interval.tick().await;
        tracing::info!("Starting periodic full scan...");

        for dir in &state.config.watcher.watch_dirs {
            if let Err(e) = api::index_api::index_directory(dir, &state, &|_, _, _| {}) {
                tracing::error!("Periodic scan failed for {}: {}", dir, e);
            }
        }

        tracing::info!("Periodic full scan complete");
    }
}
