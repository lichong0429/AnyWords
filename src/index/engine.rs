// Search engine module - wraps Tantivy for AnyWords
// Provides: index creation, document add/delete, full-text search with advanced features

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tantivy::collector::{Count, TopDocs};
use tantivy::query::{BooleanQuery, Occur, PhraseQuery, QueryParser, RegexQuery, TermQuery, RangeQuery};
use tantivy::schema::*;
use tantivy::{doc, DocAddress, Index, IndexReader, IndexWriter, ReloadPolicy, Searcher, TantivyDocument};
use tantivy::Term;
use tantivy_jieba::JiebaTokenizer;
use serde::{Deserialize, Serialize};

// ─── Data Structures ────────────────────────────────────────

/// Represents a search result with highlighted snippet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file_path: String,
    pub file_name: String,
    pub file_ext: String,
    pub score: f32,
    pub snippet: String,
    pub highlights: Vec<String>,
    pub modified: String,
    pub modified_ts: i64,
    pub size_bytes: u64,
    pub size_formatted: String,
}

/// Sort mode for search results
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortBy {
    Relevance,
    Date,
    Size,
    Name,
}

/// Search mode
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    /// Standard tokenized full-text search
    Fulltext,
    /// Exact phrase matching
    Phrase,
    /// Regex pattern matching
    Regex,
    /// Wildcard prefix search (ends with *)
    Wildcard,
}

/// Represents a query request
#[derive(Debug, Clone, Deserialize)]
pub struct SearchQuery {
    /// Search query string
    pub q: String,
    /// Max results (default 20)
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Pagination offset
    #[serde(default)]
    pub offset: usize,
    /// Filter by file extension (e.g., "pdf", "docx")
    #[serde(default)]
    pub file_type: Option<String>,
    /// Filter by path (substring match)
    #[serde(default)]
    pub path_filter: Option<String>,
    /// Search mode
    #[serde(default = "default_mode")]
    pub mode: SearchMode,
    /// Sort results by
    #[serde(default = "default_sort")]
    pub sort: SortBy,
    /// Date range filter (unix timestamp)
    #[serde(default)]
    pub date_from: Option<i64>,
    #[serde(default)]
    pub date_to: Option<i64>,
    /// Size range filter (bytes)
    #[serde(default)]
    pub size_min: Option<u64>,
    #[serde(default)]
    pub size_max: Option<u64>,
    /// Whether to include highlighted snippets
    #[serde(default = "default_true")]
    pub highlight: bool,
    /// Snippet context window size (chars around match)
    #[serde(default = "default_snippet_window")]
    pub snippet_window: usize,
}

fn default_limit() -> usize { 20 }
fn default_mode() -> SearchMode { SearchMode::Fulltext }
fn default_sort() -> SortBy { SortBy::Relevance }
fn default_true() -> bool { true }
fn default_snippet_window() -> usize { 80 }

/// Faceted search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetCounts {
    pub file_types: HashMap<String, u64>,
    pub date_ranges: HashMap<String, u64>,
}

/// Full search response with facets
#[derive(Debug, Clone, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total: u64,
    pub query: String,
    pub time_ms: f64,
    pub page: usize,
    pub total_pages: usize,
    pub facets: Option<FacetCounts>,
}

/// Search suggestion
#[derive(Debug, Clone, Serialize)]
pub struct SearchSuggestion {
    pub term: String,
    pub count: u64,
}

/// Index statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndexStats {
    pub total_docs: u64,
    pub index_size_bytes: u64,
    pub last_indexed: Option<String>,
}

/// Document stored in Tantivy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedDoc {
    pub file_path: String,
    pub file_name: String,
    pub file_ext: String,
    pub content: String,
    pub modified: i64,
    pub size_bytes: u64,
}

// ─── Engine ──────────────────────────────────────────────────

/// Search engine wrapping Tantivy
pub struct SearchEngine {
    index: Index,
    writer: RwLock<IndexWriter>,
    reader: IndexReader,
    index_path: PathBuf,

    // Schema fields
    pub field_path: Field,
    pub field_name: Field,
    pub field_ext: Field,
    pub field_content: Field,
    pub field_modified: Field,
    pub field_size: Field,
}

impl SearchEngine {
    /// Open an existing index or create a new one
    pub fn open_or_create(index_path: &Path) -> anyhow::Result<Self> {
        let mut schema_builder = Schema::builder();

        let text_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("jieba")
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();

        let field_path = schema_builder.add_text_field("path", STRING | STORED);
        let field_name = schema_builder.add_text_field("name", text_options.clone());
        let field_ext = schema_builder.add_text_field("ext", STRING | STORED);
        let field_content = schema_builder.add_text_field("content", text_options);
        let field_modified = schema_builder.add_i64_field("modified", INDEXED | STORED);
        let field_size = schema_builder.add_u64_field("size", STORED);

        let schema = schema_builder.build();

        let index = if index_path.join("meta.json").exists() {
            Index::open_in_dir(index_path)?
        } else {
            std::fs::create_dir_all(index_path)?;
            Index::create_in_dir(index_path, schema.clone())?
        };

        index.tokenizers().register("jieba", JiebaTokenizer {});

        let writer = index.writer(50_000_000)?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        Ok(Self {
            index,
            writer: RwLock::new(writer),
            reader,
            index_path: index_path.to_path_buf(),
            field_path,
            field_name,
            field_ext,
            field_content,
            field_modified,
            field_size,
        })
    }

    // ─── Index Operations ──────────────────────────────────

    /// Add or update a document
    pub fn index_document(&self, doc: &IndexedDoc) -> anyhow::Result<()> {
        let mut writer = self.writer.write().unwrap();
        let path_term = tantivy::Term::from_field_text(self.field_path, &doc.file_path);
        writer.delete_term(path_term);

        writer.add_document(doc!(
            self.field_path => doc.file_path.clone(),
            self.field_name => doc.file_name.clone(),
            self.field_ext => doc.file_ext.clone(),
            self.field_content => doc.content.clone(),
            self.field_modified => doc.modified,
            self.field_size => doc.size_bytes,
        ))?;
        writer.commit()?;
        Ok(())
    }

    /// Remove a document by path
    pub fn remove_document(&self, file_path: &str) -> anyhow::Result<()> {
        let mut writer = self.writer.write().unwrap();
        let path_term = tantivy::Term::from_field_text(self.field_path, file_path);
        writer.delete_term(path_term);
        writer.commit()?;
        Ok(())
    }

    /// Rebuild entire index
    pub fn rebuild(&self) -> anyhow::Result<()> {
        let mut writer = self.writer.write().unwrap();
        writer.delete_all_documents()?;
        writer.commit()?;
        tracing::info!("Index cleared for rebuild");
        Ok(())
    }

    // ─── Search ────────────────────────────────────────────

    /// Execute a full search with all features
    pub fn search(&self, query: &SearchQuery) -> anyhow::Result<SearchResponse> {
        self.reader.reload()?;
        let searcher = self.reader.searcher();

        // Build the core search query
        let search_query = self.build_query(&searcher, query)?;

        // Apply boolean filters if needed
        let final_query = self.apply_filters(search_query, query, &searcher)?;

        // Get total count first
        let total = searcher.search(&final_query, &Count)? as u64;
        let total_pages = if query.limit > 0 {
            (total as f64 / query.limit as f64).ceil() as usize
        } else {
            1
        };
        let page = if query.limit > 0 {
            query.offset / query.limit + 1
        } else {
            1
        };

        // Get results
        let top_docs = searcher.search(&final_query, &TopDocs::with_limit(query.limit + query.offset))?;

        // Build facet counts
        let facets = if !top_docs.is_empty() {
            Some(self.build_facets(&searcher, &top_docs, &final_query)?)
        } else {
            None
        };

        // Extract keywords for highlighting
        let keywords: Vec<String> = extract_keywords(&query.q);

        let results: Vec<SearchResult> = top_docs
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .filter_map(|(score, doc_addr)| {
                self.build_result(&searcher, doc_addr, score, &keywords, query)
            })
            .collect();

        // Apply client-side sort if not relevance
        let results = self.sort_results(results, &query.sort);

        Ok(SearchResponse {
            results,
            total,
            query: query.q.clone(),
            time_ms: 0.0, // Set by caller
            page,
            total_pages,
            facets,
        })
    }

    /// Build the main search query based on mode
    fn build_query(&self, searcher: &Searcher, query: &SearchQuery) -> anyhow::Result<Box<dyn tantivy::query::Query>> {
        let fields: Vec<Field> = vec![self.field_content, self.field_name];

        match query.mode {
            SearchMode::Fulltext | SearchMode::Wildcard => {
                let parser = QueryParser::for_index(searcher.index(), fields);
                Ok(Box::new(parser.parse_query(&query.q)?))
            }
            SearchMode::Phrase => {
                let terms: Vec<Term> = query.q.split_whitespace()
                    .map(|t| Term::from_field_text(self.field_content, t))
                    .collect();
                if terms.is_empty() {
                    return Err(anyhow::anyhow!("Empty phrase query"));
                }
                Ok(Box::new(PhraseQuery::new(terms)))
            }
            SearchMode::Regex => {
                let pattern = if query.q.starts_with('/') && query.q.ends_with('/') {
                    &query.q[1..query.q.len()-1]
                } else {
                    &query.q
                };
                let re_query = RegexQuery::from_pattern(pattern, self.field_content)?;
                Ok(Box::new(re_query))
            }
        }
    }

    /// Apply range/type/path filters on top of the search query
    fn apply_filters(
        &self,
        base: Box<dyn tantivy::query::Query>,
        query: &SearchQuery,
        searcher: &Searcher,
    ) -> anyhow::Result<Box<dyn tantivy::query::Query>> {
        let mut subqueries: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();

        // File type filter
        if let Some(ref ft) = query.file_type {
            if !ft.is_empty() {
                let term = tantivy::Term::from_field_text(self.field_ext, ft);
                subqueries.push((Occur::Must, Box::new(TermQuery::new(term, Default::default()))));
            }
        }

        // Path filter
        if let Some(ref pf) = query.path_filter {
            if !pf.is_empty() {
                let escaped = pf.replace(' ', "\\ ").replace(':', "\\:");
                let parser = QueryParser::for_index(searcher.index(), vec![self.field_path]);
                if let Ok(pq) = parser.parse_query(&escaped) {
                    subqueries.push((Occur::Must, Box::new(pq)));
                }
            }
        }

        // Date range filter
        if query.date_from.is_some() || query.date_to.is_some() {
            let from = query.date_from.unwrap_or(0);
            let to = query.date_to.unwrap_or(i64::MAX);
            let range = RangeQuery::new_i64(
                "modified".to_string(),
                from..to,
            );
            subqueries.push((Occur::Must, Box::new(range)));
        }

        // Size range filter
        if query.size_min.is_some() || query.size_max.is_some() {
            let from = query.size_min.unwrap_or(0);
            let to = query.size_max.unwrap_or(u64::MAX);
            let range = RangeQuery::new_u64(
                "size".to_string(),
                from..to,
            );
            subqueries.push((Occur::Must, Box::new(range)));
        }

        if subqueries.is_empty() {
            return Ok(base);
        }

        let mut combined: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::with_capacity(subqueries.len() + 1);
        combined.push((Occur::Must, base));
        combined.extend(subqueries);

        Ok(Box::new(BooleanQuery::new(combined)))
    }

    /// Build a single search result from a Tantivy document
    fn build_result(
        &self,
        searcher: &Searcher,
        doc_addr: DocAddress,
        score: f32,
        keywords: &[String],
        query: &SearchQuery,
    ) -> Option<SearchResult> {
        let doc: TantivyDocument = searcher.doc(doc_addr).ok()?;

        let file_path = doc.get_first(self.field_path)?.as_str()?.to_string();
        let file_name = doc.get_first(self.field_name)?.as_str()?.to_string();
        let file_ext = doc.get_first(self.field_ext)?.as_str()?.to_string();
        let size_bytes = doc.get_first(self.field_size)?.as_u64()?;
        let modified_ts = doc.get_first(self.field_modified)?.as_i64().unwrap_or(0);
        let modified = if modified_ts > 0 {
            chrono::DateTime::from_timestamp(modified_ts, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        let content = doc.get_first(self.field_content)?.as_str()?.to_string();

        let (snippet, highlights) = if query.highlight && !keywords.is_empty() {
            build_highlight_snippet(&content, keywords, query.snippet_window)
        } else {
            let s = truncate_snippet(&content, 200);
            (s, vec![])
        };

        Some(SearchResult {
            file_path,
            file_name,
            file_ext,
            score,
            snippet,
            highlights,
            modified,
            modified_ts,
            size_bytes,
            size_formatted: format_size(size_bytes),
        })
    }

    /// Sort results by the specified mode
    fn sort_results(&self, mut results: Vec<SearchResult>, sort: &SortBy) -> Vec<SearchResult> {
        match sort {
            SortBy::Relevance => results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)),
            SortBy::Date => results.sort_by(|a, b| b.modified_ts.cmp(&a.modified_ts)),
            SortBy::Size => results.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes)),
            SortBy::Name => results.sort_by(|a, b| a.file_name.cmp(&b.file_name)),
        }
        results
    }

    /// Build facet counts (file types, date ranges) from search results
    fn build_facets(
        &self,
        searcher: &Searcher,
        top_docs: &[(f32, DocAddress)],
        _query: &Box<dyn tantivy::query::Query>,
    ) -> anyhow::Result<FacetCounts> {
        let mut file_types: HashMap<String, u64> = HashMap::new();
        let mut date_ranges: HashMap<String, u64> = HashMap::new();

        let now = chrono::Utc::now().timestamp();

        for (_, doc_addr) in top_docs.iter().take(500) {
            if let Ok(doc) = searcher.doc::<TantivyDocument>(*doc_addr) {
                // File type facet
                if let Some(ext) = doc.get_first(self.field_ext).and_then(|v| v.as_str()) {
                    *file_types.entry(ext.to_string()).or_default() += 1;
                }
                // Date range facet
                if let Some(ts) = doc.get_first(self.field_modified).and_then(|v| v.as_i64()) {
                    let age = now - ts;
                    let bucket = if age < 86400 { "今天" }
                        else if age < 604800 { "本周" }
                        else if age < 2592000 { "本月" }
                        else if age < 7776000 { "三个月内" }
                        else if age < 31536000 { "一年内" }
                        else { "更早" };
                    *date_ranges.entry(bucket.to_string()).or_default() += 1;
                }
            }
        }

        Ok(FacetCounts { file_types, date_ranges })
    }

    /// Get stats with optional progress info
    pub fn stats(&self) -> anyhow::Result<IndexStats> {
        self.reader.reload()?;
        let searcher = self.reader.searcher();
        let total_docs = searcher.num_docs();

        let mut index_size: u64 = 0;
        if self.index_path.exists() {
            for entry in walkdir::WalkDir::new(&self.index_path) {
                if let Ok(entry) = entry {
                    if entry.file_type().is_file() {
                        index_size += entry.metadata().map(|m| m.len()).unwrap_or(0);
                    }
                }
            }
        }

        Ok(IndexStats {
            total_docs,
            index_size_bytes: index_size,
            last_indexed: None,
        })
    }

    /// Get search suggestions based on prefix
    pub fn suggest(&self, prefix: &str, limit: usize) -> anyhow::Result<Vec<SearchSuggestion>> {
        self.reader.reload()?;
        let searcher = self.reader.searcher();

        if prefix.len() < 2 {
            return Ok(vec![]);
        }

        let mut suggestions: Vec<SearchSuggestion> = vec![];

        // Use a simple prefix query on content field
        let parser = QueryParser::for_index(searcher.index(), vec![self.field_content]);
        if let Ok(query) = parser.parse_query(&format!("{}*", prefix)) {
            let top = searcher.search(&query, &TopDocs::with_limit(limit))?;
            for (score, doc_addr) in top {
                if let Ok(doc) = searcher.doc::<TantivyDocument>(doc_addr) {
                    if let Some(content) = doc.get_first(self.field_content).and_then(|v| v.as_str()) {
                        // Extract words near the match
                        if let Some(pos) = content.find(prefix) {
                            let start = pos.saturating_sub(10);
                            let end = (pos + prefix.len() + 40).min(content.len());
                            let snippet = if start > 0 { "..." } else { "" };
                            let snippet = format!("{}{}",
                                snippet,
                                &content[start..end].replace('\n', " ")
                            );
                            suggestions.push(SearchSuggestion {
                                term: snippet.trim().to_string(),
                                count: (score * 1000.0) as u64,
                            });
                        }
                    }
                }
            }
        }

        suggestions.dedup_by(|a, b| a.term == b.term);
        suggestions.truncate(limit);
        Ok(suggestions)
    }

    /// Get file preview with keyword highlighting
    pub fn preview(&self, file_path: &str, max_len: usize) -> anyhow::Result<Option<String>> {
        self.reader.reload()?;
        let searcher = self.reader.searcher();

        let path_term = tantivy::Term::from_field_text(self.field_path, file_path);
        let term_query = TermQuery::new(path_term, Default::default());

        let results = searcher.search(&term_query, &TopDocs::with_limit(1))?;
        if let Some((_, doc_addr)) = results.first() {
            let doc: TantivyDocument = searcher.doc(*doc_addr)?;
            if let Some(content) = doc.get_first(self.field_content).and_then(|v| v.as_str()) {
                if content.len() <= max_len {
                    return Ok(Some(content.to_string()));
                }
                return Ok(Some(format!("{}...", &content[..max_len])));
            }
        }

        Ok(None)
    }

    /// Get a field value from a document by path
    fn get_doc_field(&self, file_path: &str, field: Field) -> anyhow::Result<Option<String>> {
        self.reader.reload()?;
        let searcher = self.reader.searcher();
        let path_term = tantivy::Term::from_field_text(self.field_path, file_path);
        let query = TermQuery::new(path_term, Default::default());
        let results = searcher.search(&query, &TopDocs::with_limit(1))?;
        if let Some((_, addr)) = results.first() {
            let doc: TantivyDocument = searcher.doc(*addr)?;
            Ok(doc.get_first(field).and_then(|v| v.as_str()).map(|s| s.to_string()))
        } else {
            Ok(None)
        }
    }
}

// ─── Utility Functions ──────────────────────────────────────

/// Format bytes to human-readable size
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.1} {}", size, UNITS[unit_idx])
}

/// Extract search keywords from query string
fn extract_keywords(query: &str) -> Vec<String> {
    // Remove special Lucene operators and extract clean terms
    let cleaned = query
        .replace(" AND ", " ")
        .replace(" OR ", " ")
        .replace(" NOT ", " ")
        .replace('(', " ")
        .replace(')', " ")
        .replace('"', " ")
        .replace('*', " ")
        .replace("\\", "")
        .replace("  ", " ");

    cleaned
        .split_whitespace()
        .filter(|w| w.len() >= 2 && !w.starts_with(':') && !w.contains(':'))
        .map(|w| w.to_string())
        .collect()
}

/// Build highlighted snippet with context around keyword matches
fn build_highlight_snippet(
    content: &str,
    keywords: &[String],
    window: usize,
) -> (String, Vec<String>) {
    let clean: String = content.chars()
        .map(|c| if c.is_control() && c != '\n' && c != '\t' { ' ' } else { c })
        .collect();

    // Find first keyword match position
    let mut best_pos = None;
    for kw in keywords {
        if let Some(pos) = clean.to_lowercase().find(&kw.to_lowercase()) {
            best_pos = Some(best_pos.map_or(pos, |best: usize| best.min(pos)));
        }
    }

    let pos = best_pos.unwrap_or(0);
    let mut start = pos.saturating_sub(window);
    let mut end = (pos + window).min(clean.len());
    // Ensure we don't slice inside a multi-byte char
    while start > 0 && !clean.is_char_boundary(start) { start -= 1; }
    while !clean.is_char_boundary(end) && end < clean.len() { end += 1; }

    let snippet = if start > 0 {
        format!("...{}", &clean[start..end])
    } else {
        clean[..end].to_string()
    };

    // Generate individual highlights
    let highlights: Vec<String> = keywords.iter()
        .filter_map(|kw| {
            clean.to_lowercase()
                .match_indices(&kw.to_lowercase())
                .next()
                .map(|(p, _)| {
                    let mut h_start = p.saturating_sub(20);
                    let mut h_end = (p + kw.len() + 40).min(clean.len());
                    // Ensure we don't slice inside a multi-byte char
                    while h_start > 0 && !clean.is_char_boundary(h_start) { h_start -= 1; }
                    while !clean.is_char_boundary(h_end) && h_end < clean.len() { h_end += 1; }
                    let prefix = if h_start > 0 { "..." } else { "" };
                    let suffix = if h_end < clean.len() { "..." } else { "" };
                    let marked = format!(
                        "{}[[{}]]{}",
                        &clean[h_start..p],
                        &clean[p..p + kw.len()],
                        &clean[p + kw.len()..h_end]
                    );
                    format!("{}{}{}", prefix, marked, suffix)
                })
        })
        .take(5)
        .collect();

    (snippet, highlights)
}

/// Truncate a snippet to max_len characters
fn truncate_snippet(content: &str, max_len: usize) -> String {
    let clean: String = content.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect();

    if clean.len() <= max_len {
        return clean;
    }

    if let Some(idx) = clean[..max_len].rfind(|c: char| c.is_whitespace() || c == '。' || c == '，') {
        if idx > max_len / 2 {
            return format!("{}...", &clean[..idx]);
        }
    }

    format!("{}...", &clean[..max_len])
}
