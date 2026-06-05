// MCP (Model Context Protocol) server implementation
// Implements JSON-RPC 2.0 over HTTP POST (Streamable HTTP transport)
// This allows AI Agents (e.g. WorkBuddy, Claude Desktop, Cursor) to directly
// call AnyWords search and indexing capabilities as MCP tools.
//
// Endpoint: POST /mcp
// Content-Type: application/json
// Returns: application/json (JSON-RPC response)
//
// For SSE streaming: GET /mcp/sse (returns server-sent events stream)

use std::sync::Arc;
use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Sse},
    http::StatusCode,
};
use axum::response::sse::{Event, KeepAlive};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::convert::Infallible;

use crate::AppState;
use crate::index::engine::SearchQuery;
use crate::index::engine::SortBy;
use crate::index::engine::SearchMode;

// ─── JSON-RPC types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    fn ok(id: Option<Value>, result: Value) -> Self {
        Self { jsonrpc: "2.0".to_string(), id, result: Some(result), error: None }
    }
    fn err(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError { code, message: message.into(), data: None }),
        }
    }
}

// ─── MCP Tool definitions ────────────────────────────────────────────────────

/// Return the MCP tools list
fn mcp_tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "search",
                "description": "Full-text search across all indexed local files. Supports Chinese and English. Returns matching file paths, snippets, and relevance scores.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "q": {
                            "type": "string",
                            "description": "Search query (supports Chinese, English, keywords)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Max results to return (default: 10, max: 50)",
                            "default": 10
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Pagination offset (default: 0)",
                            "default": 0
                        },
                        "file_type": {
                            "type": "string",
                            "description": "Filter by file extension, e.g. 'pdf', 'docx', 'txt'"
                        },
                        "path_filter": {
                            "type": "string",
                            "description": "Filter results by path substring"
                        },
                        "mode": {
                            "type": "string",
                            "enum": ["fulltext", "phrase", "regex", "wildcard"],
                            "description": "Search mode (default: fulltext)",
                            "default": "fulltext"
                        }
                    },
                    "required": ["q"]
                }
            },
            {
                "name": "index_stats",
                "description": "Get statistics about the current index: number of indexed documents, index size, and last indexing time.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "scan_directory",
                "description": "Scan a local directory and add all supported files to the index in the background. Returns immediately. Use index_stats to monitor progress. Supported formats: PDF, DOCX, XLSX, PPTX, TXT, MD, and more.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "directory": {
                            "type": "string",
                            "description": "Absolute path to the directory to index"
                        }
                    },
                    "required": ["directory"]
                }
            },
            {
                "name": "get_file_content",
                "description": "Get the text content of a specific file from the index (by file path). Useful to read document contents for further analysis.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Absolute path of the file to retrieve"
                        },
                        "max_chars": {
                            "type": "integer",
                            "description": "Max characters to return (default: 5000)",
                            "default": 5000
                        }
                    },
                    "required": ["path"]
                }
            }
        ]
    })
}

/// Return the MCP server info / capabilities
fn mcp_server_info() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "anywords",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "AnyWords local file full-text search engine - search your local documents from AI agents"
        }
    })
}

// ─── Tool implementations ────────────────────────────────────────────────────

async fn tool_search(state: &AppState, params: &Value) -> Value {
    let q = match params.get("q").and_then(|v| v.as_str()) {
        Some(q) => q.to_string(),
        None => return json!({"error": "Missing required parameter 'q'"}),
    };

    let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(10).min(50) as usize;
    let offset = params.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let file_type = params.get("file_type").and_then(|v| v.as_str()).map(|s| s.to_string());
    let path_filter = params.get("path_filter").and_then(|v| v.as_str()).map(|s| s.to_string());
    let mode_str = params.get("mode").and_then(|v| v.as_str()).unwrap_or("fulltext");

    let mode = match mode_str {
        "phrase" => SearchMode::Phrase,
        "regex" => SearchMode::Regex,
        "wildcard" => SearchMode::Wildcard,
        _ => SearchMode::Fulltext,
    };

    let query = SearchQuery {
        q: q.clone(),
        limit,
        offset,
        file_type,
        path_filter,
        mode,
        sort: SortBy::Relevance,
        date_from: None,
        date_to: None,
        size_min: None,
        size_max: None,
        highlight: true,
        snippet_window: 200,
    };

    match state.engine.search(&query) {
        Ok(response) => {
            let results: Vec<Value> = response.results.iter().map(|r| {
                json!({
                    "path": r.file_path,
                    "filename": r.file_name,
                    "extension": r.file_ext,
                    "score": r.score,
                    "snippet": r.snippet,
                    "modified": r.modified,
                    "size_bytes": r.size_bytes,
                    "size": r.size_formatted
                })
            }).collect();

            json!({
                "total": response.total,
                "count": results.len(),
                "query": q,
                "elapsed_ms": response.time_ms,
                "results": results
            })
        }
        Err(e) => json!({"error": format!("Search failed: {}", e)}),
    }
}

async fn tool_index_stats(state: &AppState) -> Value {
    match state.engine.stats() {
        Ok(stats) => json!({
            "total_documents": stats.total_docs,
            "index_size_bytes": stats.index_size_bytes,
            "index_size_mb": stats.index_size_bytes / 1024 / 1024,
            "last_indexed": stats.last_indexed
        }),
        Err(e) => json!({"error": format!("Failed to get stats: {}", e)}),
    }
}

async fn tool_scan_directory(state: &Arc<AppState>, params: &Value) -> Value {
    let directory = match params.get("directory").and_then(|v| v.as_str()) {
        Some(d) => d.to_string(),
        None => return json!({"error": "Missing required parameter 'directory'"}),
    };

    // Spawn indexing as a background task so we don't block the MCP response
    let state_clone = state.clone();
    let dir_clone = directory.clone();
    tokio::spawn(async move {
        match crate::api::index_api::index_directory(&dir_clone, &state_clone, &|_, _, _| {}) {
            Ok((indexed, errors)) => {
                tracing::info!("MCP scan_directory complete: {} indexed, {} errors in {}", indexed, errors, dir_clone);
            }
            Err(e) => {
                tracing::error!("MCP scan_directory failed for {}: {}", dir_clone, e);
            }
        }
    });

    json!({
        "success": true,
        "directory": directory,
        "message": format!("Indexing started for '{}'. Use index_stats to check progress.", directory),
        "note": "Indexing runs in background. Call index_stats to monitor document count."
    })
}

async fn tool_get_file_content(state: &AppState, params: &Value) -> Value {
    let path = match params.get("path").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => return json!({"error": "Missing required parameter 'path'"}),
    };
    let max_chars = params.get("max_chars").and_then(|v| v.as_u64()).unwrap_or(5000) as usize;

    match state.engine.preview(&path, max_chars) {
        Ok(Some(content)) => json!({
            "path": path,
            "content": content,
            "truncated": content.len() >= max_chars
        }),
        Ok(None) => json!({"error": format!("File not found in index: {}", path)}),
        Err(e) => json!({"error": format!("Failed to read file: {}", e)}),
    }
}

// ─── Main HTTP handler ───────────────────────────────────────────────────────

/// Main MCP endpoint - handles JSON-RPC requests via HTTP POST
/// POST /mcp
pub async fn handle_mcp(
    State(state): State<Arc<AppState>>,
    Json(req): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let id = req.id.clone();
    let params = req.params.unwrap_or(Value::Null);

    let result = match req.method.as_str() {
        // ── MCP lifecycle ──────────────────────────────
        "initialize" => {
            JsonRpcResponse::ok(id, mcp_server_info())
        }
        "initialized" => {
            // Notification - no response needed, but return ok anyway
            JsonRpcResponse::ok(id, json!({}))
        }
        "ping" => {
            JsonRpcResponse::ok(id, json!({}))
        }

        // ── Tools ──────────────────────────────────────
        "tools/list" => {
            JsonRpcResponse::ok(id, mcp_tools_list())
        }
        "tools/call" => {
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let tool_args = params.get("arguments").unwrap_or(&Value::Null);

            let tool_result = match tool_name {
                "search" => tool_search(&state, tool_args).await,
                "index_stats" => tool_index_stats(&state).await,
                "scan_directory" => tool_scan_directory(&state, tool_args).await,
                "get_file_content" => tool_get_file_content(&state, tool_args).await,
                _ => json!({"error": format!("Unknown tool: {}", tool_name)}),
            };

            // MCP wraps tool result in content array
            let content = json!([{
                "type": "text",
                "text": serde_json::to_string_pretty(&tool_result).unwrap_or_default()
            }]);

            JsonRpcResponse::ok(id, json!({ "content": content }))
        }

        // ── Resources (not implemented, required by spec) ──
        "resources/list" => {
            JsonRpcResponse::ok(id, json!({ "resources": [] }))
        }
        "prompts/list" => {
            JsonRpcResponse::ok(id, json!({ "prompts": [] }))
        }

        // ── Unknown method ──────────────────────────────
        method => {
            JsonRpcResponse::err(id, -32601, format!("Method not found: {}", method))
        }
    };

    (StatusCode::OK, Json(result))
}

/// SSE endpoint for MCP streaming transport (GET /mcp/sse)
/// Some clients (e.g. older Claude Desktop) use SSE transport.
/// This returns a minimal SSE stream that sends the endpoint URL.
pub async fn handle_mcp_sse(
    State(_state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Send the endpoint event so the client knows where to POST
    let stream = stream::once(async {
        Ok::<Event, Infallible>(
            Event::default()
                .event("endpoint")
                .data("/mcp")
        )
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
