// Search API handlers - advanced search, suggestions, preview, export

use std::sync::Arc;
use axum::{Json, extract::State, extract::Query, response::IntoResponse, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::index::engine::SearchQuery;

/// Prettified error response
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    code: u16,
}

/// Handle search requests (GET /api/search?q=...&mode=fulltext&sort=relevance&...)
pub async fn handle_search(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let query_str = query.q.clone();

    if query_str.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Query string 'q' is required".to_string(),
                code: 400,
            }),
        ).into_response();
    }

    match state.engine.search(&query) {
        Ok(mut response) => {
            response.time_ms = start.elapsed().as_secs_f64() * 1000.0;
            (
                StatusCode::OK,
                Json(response),
            ).into_response()
        }
        Err(e) => {
            tracing::error!("Search error for '{}': {}", query_str, e);
            let msg = if e.to_string().contains("regex") {
                format!("Invalid regex pattern: {}", e)
            } else {
                format!("Search failed: {}", e)
            };
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse { error: msg, code: 400 }),
            ).into_response()
        }
    }
}

/// Handle search suggestions (GET /api/search/suggest?q=...)
pub async fn handle_suggest(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SuggestQuery>,
) -> impl IntoResponse {
    match state.engine.suggest(&params.q, params.limit.unwrap_or(10)) {
        Ok(suggestions) => (
            StatusCode::OK,
            Json(suggestions),
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e.to_string(), code: 500 }),
        ).into_response(),
    }
}

/// Handle file preview (GET /api/preview?path=...)
pub async fn handle_preview(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PreviewQuery>,
) -> impl IntoResponse {
    match state.engine.preview(&params.path, params.max_len.unwrap_or(5000)) {
        Ok(Some(content)) => (
            StatusCode::OK,
            Json(PreviewResponse { path: params.path, content }),
        ).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "File not found in index".to_string(),
                code: 404,
            }),
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e.to_string(), code: 500 }),
        ).into_response(),
    }
}

/// Handle search results export (GET /api/search/export?q=...&format=json|csv&limit=1000)
pub async fn handle_export(
    State(state): State<Arc<AppState>>,
    Query(mut query): Query<SearchQuery>,
) -> impl IntoResponse {
    // Force larger limit for export
    query.limit = query.limit.min(10000);
    query.offset = 0;

    match state.engine.search(&query) {
        Ok(response) => {
            let csv = results_to_csv(&response.results);
            (
                StatusCode::OK,
                [
                    ("Content-Type", "text/csv; charset=utf-8"),
                    ("Content-Disposition", "attachment; filename=docseek_export.csv"),
                ],
                csv,
            ).into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: e.to_string(), code: 400 }),
        ).into_response(),
    }
}

// ─── Health Check ──────────────────────────────────────────

pub async fn handle_health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

// ─── DTOs ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SuggestQuery {
    pub q: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct PreviewQuery {
    pub path: String,
    pub max_len: Option<usize>,
}

#[derive(Debug, Serialize)]
struct PreviewResponse {
    path: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

// ─── Utilities ─────────────────────────────────────────────

fn results_to_csv(results: &[crate::index::engine::SearchResult]) -> String {
    let mut csv = String::from("文件路径,文件名,类型,相关度,修改时间,大小\n");
    for r in results {
        csv.push_str(&format!(
            "\"{}\",\"{}\",\"{}\",{:.2},\"{}\",\"{}\"\n",
            r.file_path.replace('"', "\"\""),
            r.file_name.replace('"', "\"\""),
            r.file_ext,
            r.score,
            r.modified,
            r.size_formatted,
        ));
    }
    csv
}
