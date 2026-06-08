// API router - defines all REST endpoints

use std::sync::Arc;
use axum::{Router, routing::get, routing::post};
use crate::AppState;

use super::search::*;
use super::index_api::*;
use super::mcp::{handle_mcp, handle_mcp_sse};

/// Create the main API router
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // ── Search ────────────────────────────────────
        .route("/api/search", get(handle_search))
        .route("/api/search", post(handle_search))
        .route("/api/search/suggest", get(handle_suggest))
        .route("/api/search/export", get(handle_export))
        .route("/api/preview", get(handle_preview))

        // ── Index management ──────────────────────────
        .route("/api/index/stats", get(handle_index_stats))
        .route("/api/index/progress", get(handle_index_progress))
        .route("/api/index/rebuild", post(handle_index_rebuild))
        .route("/api/index/add", post(handle_index_add))
        .route("/api/index/remove", post(handle_index_remove))
        .route("/api/index/scan", post(handle_index_scan))

        // ── Directory browsing ────────────────────────
        .route("/api/browse", get(handle_browse))
        .route("/api/roots", get(handle_roots))

        // ── MCP (Model Context Protocol) ──────────────
        // HTTP POST transport (standard)
        .route("/mcp", post(handle_mcp))
        // SSE transport (for legacy clients like older Claude Desktop)
        .route("/mcp/sse", get(handle_mcp_sse))

        // ── Health ────────────────────────────────────
        .route("/api/health", get(handle_health))

        .with_state(state)
}
