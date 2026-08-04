// API types for AnyWords

export interface SearchResult {
  file_path: string;
  file_name: string;
  file_ext: string;
  score: number;
  snippet: string;
  highlights: string[];
  modified: string;
  modified_ts: number;
  size_bytes: number;
  size_formatted: string;
}

export interface FacetCounts {
  file_types: Record<string, number>;
  date_ranges: Record<string, number>;
}

export interface SearchResponse {
  results: SearchResult[];
  total: number;
  query: string;
  time_ms: number;
  page: number;
  total_pages: number;
  facets: FacetCounts | null;
}

export type SearchMode = 'fulltext' | 'phrase' | 'regex' | 'wildcard';
export type SortBy = 'relevance' | 'date' | 'size' | 'name';

export interface SearchQuery {
  q: string;
  limit?: number;
  offset?: number;
  file_type?: string;
  path_filter?: string;
  mode?: SearchMode;
  sort?: SortBy;
  date_from?: number;
  date_to?: number;
  size_min?: number;
  size_max?: number;
  highlight?: boolean;
  snippet_window?: number;
}

export interface IndexStats {
  total_docs: number;
  index_size_bytes: number;
  last_indexed: string | null;
}

export interface IndexProgress {
  current: number;
  total: number;
  message: string;
  percent: number;
}

export interface IndexOpResponse {
  success: boolean;
  message: string;
  count?: number;
  errors?: number;
}

// ─── Application configuration (mirrors anywords.yml) ───

export interface ServerConfig {
  port: number;
  host: string;
}

export interface IndexConfig {
  dir: string;
  writer_buffer_bytes: number;
  max_file_size_bytes: number;
}

export interface ParserConfig {
  tika_server_url: string | null;
  tika_jar_path: string | null;
  fallback_basic: boolean;
}

export interface WatcherConfig {
  watch_dirs: string[];
  enabled: boolean;
  debounce_ms: number;
  full_scan_interval_secs: number;
  exclude_extensions: string[];
  include_extensions: string[];
  exclude_patterns: string[];
}

export interface LogConfig {
  level: string;
}

export interface AppConfig {
  server: ServerConfig;
  index: IndexConfig;
  parser: ParserConfig;
  watcher: WatcherConfig;
  logging: LogConfig;
}
