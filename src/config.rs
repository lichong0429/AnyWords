// Configuration module
// Loads config from YAML file, supports CLI override via env vars

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// HTTP server settings
    #[serde(default)]
    pub server: ServerConfig,

    /// Search engine settings
    #[serde(default)]
    pub index: IndexConfig,

    /// Document parsing settings
    #[serde(default)]
    pub parser: ParserConfig,

    /// File watching settings
    #[serde(default)]
    pub watcher: WatcherConfig,

    /// Logging settings
    #[serde(default)]
    pub logging: LogConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_host")]
    pub host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    /// Directory to store index files
    #[serde(default = "default_index_dir")]
    pub dir: PathBuf,
    /// Writer buffer size in bytes (default 50MB)
    #[serde(default = "default_writer_buffer")]
    pub writer_buffer_bytes: usize,
    /// Maximum file size to index (default 50MB)
    #[serde(default = "default_max_file_size")]
    pub max_file_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserConfig {
    /// Tika server URL (e.g., http://localhost:9998)
    #[serde(default)]
    pub tika_server_url: Option<String>,
    /// Tika JAR path (for local Java invocation)
    #[serde(default)]
    pub tika_jar_path: Option<String>,
    /// Fallback to basic parser if Tika is unavailable
    #[serde(default = "default_true")]
    pub fallback_basic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherConfig {
    /// Directories to watch for changes
    #[serde(default)]
    pub watch_dirs: Vec<String>,
    /// Enable file change monitoring
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Debounce delay in milliseconds
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    /// Scan interval for periodic full sync (in seconds, 0 = disabled)
    #[serde(default)]
    pub full_scan_interval_secs: u64,
    /// File extensions to exclude from watching
    #[serde(default = "default_exclude_extensions")]
    pub exclude_extensions: Vec<String>,
    /// Glob patterns to exclude
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log level: trace, debug, info, warn, error
    #[serde(default = "default_log_level")]
    pub level: String,
}

// Default values
fn default_port() -> u16 { 9921 }
fn default_host() -> String { "127.0.0.1".to_string() }
fn default_index_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("anywords")
        .join("index")
}
fn default_writer_buffer() -> usize { 50_000_000 }
fn default_max_file_size() -> u64 { 50 * 1024 * 1024 }
fn default_debounce_ms() -> u64 { 2000 }
fn default_log_level() -> String { "info".to_string() }
fn default_true() -> bool { true }
fn default_exclude_extensions() -> Vec<String> {
    vec![
        "tmp".into(), "temp".into(), "lock".into(),
        "sys".into(), "dll".into(), "exe".into(),
        "so".into(), "dylib".into(), "class".into(),
        "o".into(), "obj".into(), "a".into(),
    ]
}
fn default_exclude_patterns() -> Vec<String> {
    vec![
        "node_modules".into(), ".git".into(), "__pycache__".into(),
        ".idea".into(), ".vscode".into(), "target".into(),
        ".cargo".into(), ".rustup".into(), "vendor".into(),
    ]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            index: IndexConfig::default(),
            parser: ParserConfig::default(),
            watcher: WatcherConfig::default(),
            logging: LogConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            host: default_host(),
        }
    }
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            dir: default_index_dir(),
            writer_buffer_bytes: default_writer_buffer(),
            max_file_size_bytes: default_max_file_size(),
        }
    }
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            tika_server_url: None,
            tika_jar_path: None,
            fallback_basic: true,
        }
    }
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            watch_dirs: Vec::new(),
            enabled: true,
            debounce_ms: default_debounce_ms(),
            full_scan_interval_secs: 0,
            exclude_extensions: default_exclude_extensions(),
            exclude_patterns: default_exclude_patterns(),
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

impl Config {
    /// Load config from YAML file, merging with defaults
    pub fn load(config_path: Option<&str>) -> anyhow::Result<Self> {
        let path = config_path.unwrap_or("anywords.yml");

        let config = if std::path::Path::new(path).exists() {
            let content = std::fs::read_to_string(path)?;
            let mut cfg: Config = serde_yaml::from_str(&content)?;

            // Resolve relative paths
            cfg.index.dir = resolve_path(&cfg.index.dir);
            cfg.watcher.watch_dirs = cfg.watcher.watch_dirs.iter()
                .map(|d| resolve_path(&PathBuf::from(d)).to_string_lossy().to_string())
                .collect();
            if let Some(ref url) = cfg.parser.tika_server_url {
                if url.is_empty() { cfg.parser.tika_server_url = None; }
            }
            if let Some(ref jar) = cfg.parser.tika_jar_path {
                if jar.is_empty() { cfg.parser.tika_jar_path = None; }
            }

            cfg
        } else {
            tracing::info!("No config file found at '{}', using defaults", path);
            let cfg = Config::default();
            // Create default config file for reference
            if let Ok(yaml) = serde_yaml::to_string(&cfg) {
                if let Err(e) = std::fs::write(path, &yaml) {
                    tracing::warn!("Failed to write default config: {}", e);
                } else {
                    tracing::info!("Default config written to '{}'", path);
                }
            }
            cfg
        };

        Ok(config)
    }
}

/// Resolve path: if relative, make it relative to the config file location
/// For simplicity, relative paths are relative to current dir
fn resolve_path(path: &PathBuf) -> PathBuf {
    if path.is_absolute() {
        path.clone()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}
