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
    let watcher_enabled = state.config.read()
        .map(|c| c.watcher.enabled)
        .unwrap_or(true);
    if watcher_enabled {
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

// ─── Full Configuration API ─────────────────────────────

/// GET /api/config
/// Returns the live configuration (as loaded from anywords.yml plus any
/// runtime changes made via POST /api/config).
pub async fn handle_get_config(
    State(state): State<Arc<AppState>>,
) -> Json<crate::config::Config> {
    let cfg = state.config.read().map(|c| c.clone()).unwrap_or_default();
    Json(cfg)
}

/// POST /api/config
/// Replaces the configuration (except watch_dirs, which is managed by its
/// own endpoints), persists it to anywords.yml, and applies the per-file
/// indexing rules immediately. Changes to server / index dir / Tika /
/// logging only take effect after a restart; the response says which.
pub async fn handle_update_config(
    State(state): State<Arc<AppState>>,
    Json(mut new_cfg): Json<crate::config::Config>,
) -> Json<IndexOpResponse> {
    // watch_dirs is owned by the runtime list (and its own endpoints that
    // also trigger scans/watchers); never let this endpoint overwrite it.
    let runtime_dirs = state.watch_dirs.read().await.clone();
    new_cfg.watcher.watch_dirs = runtime_dirs;

    // ── Validation ──────────────────────────────────────
    if new_cfg.server.port == 0 {
        return Json(IndexOpResponse {
            success: false,
            message: "Invalid port: must be 1-65535".to_string(),
            count: None,
            errors: None,
        });
    }
    if new_cfg.server.host.trim().is_empty() {
        return Json(IndexOpResponse {
            success: false,
            message: "Host must not be empty".to_string(),
            count: None,
            errors: None,
        });
    }
    if new_cfg.index.max_file_size_bytes == 0 {
        return Json(IndexOpResponse {
            success: false,
            message: "Max file size must be greater than 0".to_string(),
            count: None,
            errors: None,
        });
    }
    let level = new_cfg.logging.level.to_lowercase();
    if !["trace", "debug", "info", "warn", "error"].contains(&level.as_str()) {
        return Json(IndexOpResponse {
            success: false,
            message: format!("Invalid log level: {}", new_cfg.logging.level),
            count: None,
            errors: None,
        });
    }

    // Normalize: lowercase extensions, drop blanks
    let norm = |v: &mut Vec<String>| {
        v.retain(|s| !s.trim().is_empty());
        for s in v.iter_mut() {
            *s = s.trim().trim_start_matches('.').to_lowercase();
        }
        v.sort();
        v.dedup();
    };
    norm(&mut new_cfg.watcher.exclude_extensions);
    norm(&mut new_cfg.watcher.include_extensions);
    new_cfg.watcher.exclude_patterns.retain(|s| !s.trim().is_empty());

    // Tika: empty string means "not configured"
    if new_cfg.parser.tika_server_url.as_deref().map(str::is_empty) == Some(true) {
        new_cfg.parser.tika_server_url = None;
    }
    if new_cfg.parser.tika_jar_path.as_deref().map(str::is_empty) == Some(true) {
        new_cfg.parser.tika_jar_path = None;
    }

    // ── Compare with current config to report restart-needed fields ──
    let restart_fields: Vec<&str> = {
        let old = state.config.read().map(|c| c.clone()).unwrap_or_default();
        let mut fields = Vec::new();
        if old.server.port != new_cfg.server.port || old.server.host != new_cfg.server.host {
            fields.push("服务地址/端口");
        }
        if old.index.dir != new_cfg.index.dir {
            fields.push("索引目录");
        }
        if old.index.writer_buffer_bytes != new_cfg.index.writer_buffer_bytes {
            fields.push("写入缓冲区");
        }
        if old.parser.tika_server_url != new_cfg.parser.tika_server_url
            || old.parser.tika_jar_path != new_cfg.parser.tika_jar_path
            || old.parser.fallback_basic != new_cfg.parser.fallback_basic
        {
            fields.push("Tika 解析");
        }
        if old.watcher.enabled != new_cfg.watcher.enabled
            || old.watcher.debounce_ms != new_cfg.watcher.debounce_ms
            || old.watcher.full_scan_interval_secs != new_cfg.watcher.full_scan_interval_secs
        {
            fields.push("监控开关/防抖/定时全量扫描");
        }
        if old.logging.level != new_cfg.logging.level {
            fields.push("日志级别");
        }
        fields
    };

    // ── Persist to anywords.yml ─────────────────────────
    let yaml = match serde_yaml::to_string(&new_cfg) {
        Ok(y) => y,
        Err(e) => {
            return Json(IndexOpResponse {
                success: false,
                message: format!("Failed to serialize config: {}", e),
                count: None,
                errors: None,
            });
        }
    };
    if let Err(e) = std::fs::write("anywords.yml", yaml) {
        return Json(IndexOpResponse {
            success: false,
            message: format!("Failed to write anywords.yml: {}", e),
            count: None,
            errors: None,
        });
    }

    // ── Apply to the live config (per-file rules take effect now) ──
    if let Ok(mut guard) = state.config.write() {
        *guard = new_cfg;
    }

    let message = if restart_fields.is_empty() {
        "配置已保存并即时生效（排除/包含扩展名、排除模式、单文件大小上限立即应用于新索引的文件）".to_string()
    } else {
        format!(
            "配置已保存，以下变更重启后生效: {}；其余规则已即时生效",
            restart_fields.join("、")
        )
    };

    Json(IndexOpResponse {
        success: true,
        message,
        count: None,
        errors: None,
    })
}
