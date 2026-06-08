// File system monitor
// Watches directories for file changes and triggers re-indexing

use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;
use notify::{Event, EventKind, RecursiveMode, Watcher, Config};
use tokio::sync::mpsc;

use crate::AppState;

/// Start a file watcher for the given directories
pub fn start_watcher(
    state: Arc<AppState>,
    watch_dirs: Vec<PathBuf>,
) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel::<Event>(1000);

    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res {
            let _ = tx.blocking_send(event);
        }
    })?;

    watcher.configure(
        Config::default()
            .with_poll_interval(std::time::Duration::from_secs(2))
    )?;

    // Watch each directory
    for dir in &watch_dirs {
        if dir.exists() && dir.is_dir() {
            watcher.watch(dir, RecursiveMode::Recursive)?;
            tracing::info!("Watching directory: {}", dir.display());
        } else {
            tracing::warn!("Watch directory does not exist: {}", dir.display());
        }
    }

    let debounce_ms = state.config.watcher.debounce_ms;

    // Spawn async handler with debouncing
    tokio::spawn(async move {
        let mut pending: HashMap<String, (EventKind, tokio::time::Instant)> = HashMap::new();

        loop {
            // Process events with a timeout to flush pending
            let event = tokio::time::timeout(
                std::time::Duration::from_millis(debounce_ms),
                rx.recv(),
            ).await;

            match event {
                Ok(Some(event)) => {
                    for path in &event.paths {
                        let key = path.to_string_lossy().to_string();
                        let now = tokio::time::Instant::now();

                        match pending.get(&key) {
                            Some(&(_, last_time)) if now.duration_since(last_time).as_millis() < debounce_ms as u128 => {
                                // Still in debounce window, update event
                                pending.insert(key, (event.kind, last_time));
                            }
                            _ => {
                                pending.insert(key, (event.kind, now));
                            }
                        }
                    }
                }
                Ok(None) => {
                    // Channel closed
                    break;
                }
                Err(_) => {
                    // Timeout - flush pending events
                    let now = tokio::time::Instant::now();
                    let expired: Vec<String> = pending.iter()
                        .filter(|(_, (_, t))| now.duration_since(*t).as_millis() >= debounce_ms as u128)
                        .map(|(k, _)| k.clone())
                        .collect();

                    for key in expired {
                        if let Some((kind, _)) = pending.remove(&key) {
                            let path = PathBuf::from(&key);
                            handle_file_event(&state, kind, &path).await;
                        }
                    }
                }
            }
        }
    });

    Ok(())
}

/// Handle a debounced file system event
async fn handle_file_event(state: &Arc<AppState>, kind: EventKind, path: &PathBuf) {
    // Check exclusion rules
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name.starts_with('.') || name.starts_with('~') {
            return;
        }
    }

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        if state.config.watcher.exclude_extensions.contains(&ext_lower) {
            return;
        }
        // Skip if not in include list (when whitelist is configured)
        if !state.config.watcher.include_extensions.is_empty()
            && !state.config.watcher.include_extensions.contains(&ext_lower)
        {
            return;
        }
    }

    let path_str = path.to_string_lossy().to_lowercase();
    for pattern in &state.config.watcher.exclude_patterns {
        if path_str.contains(&pattern.to_lowercase()) {
            return;
        }
    }

    match kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            if path.is_file() {
                // Small delay to let file writes finish
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                // Skip files that don't exist anymore (temporary files)
                if !path.exists() {
                    return;
                }

                match crate::api::index_api::index_single_file(state, path) {
                    Ok(_) => tracing::debug!("Re-indexed: {}", path.display()),
                    Err(e) => {
                        if !e.to_string().contains("Excluded") {
                            tracing::debug!("Skip indexing {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
        EventKind::Remove(_) => {
            let file_path = path.to_string_lossy().to_string();
            if let Err(e) = state.engine.remove_document(&file_path) {
                tracing::debug!("Failed to remove {}: {}", file_path, e);
            } else {
                tracing::debug!("Removed from index: {}", file_path);
            }
        }
        _ => {}
    }
}
