// Configuration API handlers
// Manage watched directories at runtime and persist them to anywords.yml

use std::path::{Path, PathBuf};
use std::sync::Arc;
use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::AppState;
use super::index_api::{index_directory, IndexOpResponse};

/// Response listing the currently watched directories
#[derive(Debug, Serialize)]
pub struct WatchDirsResponse {
    pub watch_dirs: Vec<String>,
}

/// Request to add/remove a watched directory
#[derive(Debug, Deserialize)]
pub struct WatchDirRequest {
    pub path: String,
}

/// GET /api/config/watch_dirs
pub async fn handle_get_watch_dirs(
    State(state): State<Arc<AppState>>,
) -> Json<WatchDirsResponse> {
    let dirs = state.watch_dirs.read().await.clone();
    Json(WatchDirsResponse { watch_dirs: dirs })
}

/// POST /api/config/watch_dirs/add
/// Adds a directory to the watch list, persists it, scans it in the
/// background and starts live file monitoring immediately.
pub async fn handle_add_watch_dir(
    State(state): State<Arc<AppState>>,
    Json(req): Json<WatchDirRequest>,
) -> Json<IndexOpResponse> {
    let raw = req.path.trim().trim_matches('"');
    if raw.is_empty() {
        return Json(IndexOpResponse {
            success: false,
            message: "Path is empty".to_string(),
            count: None,
            errors: None,
        });
    }

    let mut dir = PathBuf::from(raw);
    if !dir.is_absolute() {
        if let Ok(cwd) = std::env::current_dir() {
            dir = cwd.join(dir);
        }
    }
    if !dir.is_dir() {
        return Json(IndexOpResponse {
            success: false,
            message: format!("Directory not found: {}", raw),
            count: None,
            errors: None,
        });
    }
    let dir_str = dir.to_string_lossy().to_string();

    {
        let mut dirs = state.watch_dirs.write().await;
        if dirs.iter().any(|d| d.eq_ignore_ascii_case(&dir_str)) {
            return Json(IndexOpResponse {
                success: false,
                message: format!("Already watching: {}", dir_str),
                count: None,
                errors: None,
            });
        }
        dirs.push(dir_str.clone());
        if let Err(e) = persist_watch_dirs(&dirs) {
            dirs.pop();
            return Json(IndexOpResponse {
                success: false,
                message: format!("Failed to save config: {}", e),
                count: None,
                errors: None,
            });
        }
    }

    // Scan the new directory in the background so it does not block the UI
    let scan_state = state.clone();
    let scan_dir = dir_str.clone();
    tokio::task::spawn_blocking(move || {
        if let Err(e) = index_directory(&scan_dir, &scan_state, &|_, _, _| {}) {
            tracing::error!("Failed to scan newly added dir {}: {}", scan_dir, e);
        }
    });

    // Start live monitoring for the new directory right away
    if state.config.watcher.enabled {
        if let Err(e) = crate::watcher::monitor::start_watcher(state.clone(), vec![dir]) {
            tracing::warn!("Failed to start watcher for new dir: {}", e);
        }
    }

    Json(IndexOpResponse {
        success: true,
        message: format!("Added and indexing in background: {}", dir_str),
        count: None,
        errors: None,
    })
}

/// POST /api/config/watch_dirs/remove
/// Removes a directory from the watch list and persists the change.
/// Indexed documents are kept (use rebuild to clear them); the already
/// running live monitor for this directory stops after a restart.
pub async fn handle_remove_watch_dir(
    State(state): State<Arc<AppState>>,
    Json(req): Json<WatchDirRequest>,
) -> Json<IndexOpResponse> {
    let raw = req.path.trim().trim_matches('"');

    let mut removed: Option<String> = None;
    {
        let mut dirs = state.watch_dirs.write().await;
        if let Some(pos) = dirs.iter().position(|d| d.eq_ignore_ascii_case(raw)) {
            removed = Some(dirs.remove(pos));
            if let Err(e) = persist_watch_dirs(&dirs) {
                if let Some(r) = &removed {
                    dirs.push(r.clone());
                }
                return Json(IndexOpResponse {
                    success: false,
                    message: format!("Failed to save config: {}", e),
                    count: None,
                    errors: None,
                });
            }
        }
    }

    match removed {
        Some(dir_str) => Json(IndexOpResponse {
            success: true,
            message: format!(
                "Removed: {} (indexed files are kept; live monitoring for it stops after restart)",
                dir_str
            ),
            count: None,
            errors: None,
        }),
        None => Json(IndexOpResponse {
            success: false,
            message: format!("Not in watch list: {}", raw),
            count: None,
            errors: None,
        }),
    }
}

/// Persist the watch directory list back into anywords.yml, preserving
/// every other configuration value.
fn persist_watch_dirs(dirs: &[String]) -> anyhow::Result<()> {
    let path = Path::new("anywords.yml");

    let mut value: serde_yaml::Value = if path.exists() {
        serde_yaml::from_str(&std::fs::read_to_string(path)?)?
    } else {
        serde_yaml::to_value(crate::config::Config::default())?
    };

    let dirs_value = serde_yaml::to_value(dirs)?;
    let watcher_key = serde_yaml::Value::String("watcher".to_string());
    let watch_dirs_key = serde_yaml::Value::String("watch_dirs".to_string());

    if let serde_yaml::Value::Mapping(ref mut root) = value {
        if !root.contains_key(&watcher_key) {
            root.insert(watcher_key.clone(), serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
        }
        if let Some(serde_yaml::Value::Mapping(watcher)) = root.get_mut(&watcher_key) {
            watcher.insert(watch_dirs_key, dirs_value);
        }
    }

    std::fs::write(path, serde_yaml::to_string(&value)?)?;
    Ok(())
}
