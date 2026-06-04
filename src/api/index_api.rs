// Index management API handlers

use std::path::Path;
use std::sync::Arc;
use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::AppState;
use crate::config::Config;
use crate::index::engine::{IndexedDoc, IndexStats};
use crate::parser::extractor;

/// Request to add a single file
#[derive(Debug, Deserialize)]
pub struct AddFileRequest {
    pub file_path: String,
}

/// Request to remove a file
#[derive(Debug, Deserialize)]
pub struct RemoveFileRequest {
    pub file_path: String,
}

/// Request to scan a directory
#[derive(Debug, Deserialize)]
pub struct ScanDirRequest {
    pub directory: String,
    #[serde(default)]
    pub recursive: bool,
}

/// Progress info for indexing
#[derive(Debug, Clone, Serialize)]
pub struct IndexProgress {
    pub current: usize,
    pub total: usize,
    pub message: String,
    pub percent: f64,
}

/// Response for index operations
#[derive(Debug, Serialize)]
pub struct IndexOpResponse {
    pub success: bool,
    pub message: String,
    pub count: Option<usize>,
    pub errors: Option<usize>,
}

/// Get index statistics
pub async fn handle_index_stats(
    State(state): State<Arc<AppState>>,
) -> Json<IndexStats> {
    match state.engine.stats() {
        Ok(mut stats) => {
            // Add progress info
            let progress = state.index_progress.read().await;
            if !progress.2.is_empty() {
                stats.last_indexed = Some(format!("({}/{}) {}", progress.0, progress.1, progress.2));
            }
            Json(stats)
        }
        Err(e) => {
            tracing::error!("Stats error: {}", e);
            Json(IndexStats {
                total_docs: 0,
                index_size_bytes: 0,
                last_indexed: None,
            })
        }
    }
}

/// Rebuild the entire index
pub async fn handle_index_rebuild(
    State(state): State<Arc<AppState>>,
) -> Json<IndexOpResponse> {
    match state.engine.rebuild() {
        Ok(()) => Json(IndexOpResponse {
            success: true,
            message: "Index cleared successfully".to_string(),
            count: None,
            errors: None,
        }),
        Err(e) => Json(IndexOpResponse {
            success: false,
            message: format!("Failed to rebuild index: {}", e),
            count: None,
            errors: None,
        }),
    }
}

/// Add a single file to the index
pub async fn handle_index_add(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddFileRequest>,
) -> Json<IndexOpResponse> {
    let file_path = Path::new(&req.file_path);
    
    if !file_path.exists() {
        return Json(IndexOpResponse {
            success: false,
            message: format!("File not found: {}", req.file_path),
            count: None,
            errors: None,
        });
    }

    match index_single_file(&state, file_path) {
        Ok(_) => Json(IndexOpResponse {
            success: true,
            message: format!("Indexed: {}", req.file_path),
            count: Some(1),
            errors: None,
        }),
        Err(e) => Json(IndexOpResponse {
            success: false,
            message: format!("Failed to index: {}", e),
            count: None,
            errors: None,
        }),
    }
}

/// Remove a file from the index
pub async fn handle_index_remove(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RemoveFileRequest>,
) -> Json<IndexOpResponse> {
    match state.engine.remove_document(&req.file_path) {
        Ok(()) => Json(IndexOpResponse {
            success: true,
            message: format!("Removed: {}", req.file_path),
            count: Some(1),
            errors: None,
        }),
        Err(e) => Json(IndexOpResponse {
            success: false,
            message: format!("Failed to remove: {}", e),
            count: None,
            errors: None,
        }),
    }
}

/// Get current indexing progress
pub async fn handle_index_progress(
    State(state): State<Arc<AppState>>,
) -> Json<IndexProgress> {
    let progress = state.index_progress.read().await;
    let percent = if progress.1 > 0 {
        (progress.0 as f64 / progress.1 as f64) * 100.0
    } else {
        0.0
    };
    Json(IndexProgress {
        current: progress.0,
        total: progress.1,
        message: progress.2.clone(),
        percent,
    })
}

/// Scan a directory and index all supported files
pub async fn handle_index_scan(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ScanDirRequest>,
) -> Json<IndexOpResponse> {
    let dir = Path::new(&req.directory);
    
    if !dir.exists() || !dir.is_dir() {
        return Json(IndexOpResponse {
            success: false,
            message: format!("Directory not found: {}", req.directory),
            count: None,
            errors: None,
        });
    }

    let (counted, errors) = match index_directory(&req.directory, &state, &|_, _, _| {}) {
        Ok((i, e)) => (i, e),
        Err(e) => {
            return Json(IndexOpResponse {
                success: false,
                message: format!("Scan failed: {}", e),
                count: None,
                errors: None,
            });
        }
    };

    Json(IndexOpResponse {
        success: true,
        message: format!(
            "Indexed {} files from '{}'{}",
            counted,
            req.directory,
            if errors > 0 { format!(" ({} failed)", errors) } else { String::new() }
        ),
        count: Some(counted),
        errors: Some(errors),
    })
}

// --- Public utility functions used by main.rs and watcher ---

/// Count eligible files in a directory
pub fn count_files_in_dir(dir: &str, config: &Config) -> anyhow::Result<usize> {
    let mut count = 0;
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        if should_skip_file(entry.path(), config) {
            continue;
        }
        count += 1;
    }
    Ok(count)
}

/// Index all eligible files in a directory
/// Returns (indexed_count, error_count)
pub fn index_directory(
    dir: &str,
    state: &Arc<AppState>,
    on_progress: &dyn Fn(usize, usize, &str),
) -> anyhow::Result<(usize, usize)> {
    let mut count = 0;
    let mut errors = 0;

    // Count total
    let total = count_files_in_dir(dir, &state.config)?;
    let mut scanned = 0;

    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        if should_skip_file(path, &state.config) {
            continue;
        }

        scanned += 1;
        match index_single_file(state, path) {
            Ok(_) => {
                count += 1;
                on_progress(scanned, total, &format!("Indexed: {}", path.display()));
            }
            Err(e) => {
                errors += 1;
                // Don't spam warnings for unsupported formats
                if !e.to_string().contains("No extractable") {
                    tracing::debug!("Failed to index {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok((count, errors))
}

/// Index a single file (public, used by watcher and API)
pub fn index_single_file(state: &Arc<AppState>, file_path: &Path) -> anyhow::Result<()> {
    // Get file metadata
    let metadata = std::fs::metadata(file_path)?;
    let file_name = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let file_ext = file_path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    
    // Skip based on extension
    if state.config.watcher.exclude_extensions.contains(&file_ext) {
        return Err(anyhow::anyhow!("Excluded extension: {}", file_ext));
    }

    // Check file size
    if metadata.len() > state.config.index.max_file_size_bytes {
        return Err(anyhow::anyhow!(
            "File too large: {} bytes (max: {})",
            metadata.len(),
            state.config.index.max_file_size_bytes
        ));
    }

    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let size_bytes = metadata.len();

    // Extract text - use Tika for complex formats if available
    let extracted = extractor::extract_text_sync(file_path, state.tika.as_ref())?;

    let doc = IndexedDoc {
        file_path: file_path.to_string_lossy().to_string(),
        file_name,
        file_ext,
        content: extracted.text,
        modified,
        size_bytes,
    };

    state.engine.index_document(&doc)?;
    Ok(())
}

// --- Private helpers ---

/// Check if a file should be skipped based on config rules
fn should_skip_file(path: &Path, config: &Config) -> bool {
    // Skip hidden files
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name.starts_with('.') || name.starts_with('~') {
            return true;
        }
    }

    // Skip excluded extensions
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if config.watcher.exclude_extensions.contains(&ext.to_lowercase()) {
            return true;
        }
    }

    // Skip excluded patterns (check path components)
    let path_str = path.to_string_lossy().to_lowercase();
    for pattern in &config.watcher.exclude_patterns {
        if path_str.contains(&pattern.to_lowercase()) {
            return true;
        }
    }

    // Skip files larger than max
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() > config.index.max_file_size_bytes {
            return true;
        }
    }

    false
}
