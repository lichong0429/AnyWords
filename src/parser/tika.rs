// Apache Tika integration for enhanced document parsing
// Supports two modes:
// 1. Tika Server mode (HTTP REST API on port 9998)
// 2. Tika JAR mode (subprocess call to tika-app.jar)

use std::path::Path;
use std::process::Command;

/// Configuration for Tika integration
#[derive(Debug, Clone)]
pub struct TikaConfig {
    /// URL of a running Tika Server (e.g., http://localhost:9998)
    pub server_url: Option<String>,
    /// Path to tika-app.jar for local invocation
    pub jar_path: Option<String>,
    /// Whether to fall back to basic parser
    pub fallback: bool,
}

impl Default for TikaConfig {
    fn default() -> Self {
        Self {
            server_url: None,
            jar_path: None,
            fallback: true,
        }
    }
}

/// Tika parser wrapper
pub struct TikaParser {
    pub config: TikaConfig,
}

impl TikaParser {
    pub fn new(config: TikaConfig) -> Self {
        Self { config }
    }

    /// Check if Tika is available (either server or JAR)
    pub fn is_available(&self) -> bool {
        self.config.server_url.is_some() || self.config.jar_path.is_some()
    }

    /// Extract text from a file using Tika
    /// Returns Ok(Some(text)) on success, Ok(None) if Tika is not available,
    /// Err on actual parsing error
    pub async fn extract_text(&self, file_path: &Path) -> anyhow::Result<Option<String>> {
        // Try server mode first (faster, no JVM startup)
        if let Some(ref server_url) = self.config.server_url {
            match self.extract_via_server(file_path, server_url).await {
                Ok(text) => return Ok(Some(text)),
                Err(e) => {
                    tracing::warn!("Tika server extraction failed for {}: {}", file_path.display(), e);
                    // Fall through to JAR mode
                }
            }
        }

        // Try JAR mode
        if let Some(ref jar_path) = self.config.jar_path {
            match self.extract_via_jar(file_path, jar_path) {
                Ok(text) => return Ok(Some(text)),
                Err(e) => {
                    tracing::warn!("Tika JAR extraction failed for {}: {}", file_path.display(), e);
                }
            }
        }

        if self.config.fallback {
            Ok(None) // Signal to fall back to basic parser
        } else {
            Err(anyhow::anyhow!("Tika extraction failed and fallback is disabled"))
        }
    }

    /// Extract via HTTP Tika Server
    async fn extract_via_server(&self, file_path: &Path, server_url: &str) -> anyhow::Result<String> {
        let url = format!("{}/tika", server_url.trim_end_matches('/'));

        let file_bytes = tokio::fs::read(file_path).await?;
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let client = reqwest::Client::new();
        let part = reqwest::multipart::Part::bytes(file_bytes)
            .file_name(file_name.to_string())
            .mime_str("application/octet-stream")?;

        let form = reqwest::multipart::Form::new().part("file", part);

        let response = client.put(&url)
            .multipart(form)
            .header("Accept", "text/plain")
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Tika server returned HTTP {}: {}",
                response.status().as_u16(),
                response.text().await.unwrap_or_default()
            ));
        }

        let text = response.text().await?;
        Ok(text)
    }

    /// Extract via local Tika JAR (subprocess) - async version
    fn extract_via_jar(&self, file_path: &Path, jar_path: &str) -> anyhow::Result<String> {
        let file_str = file_path.to_string_lossy().to_string();

        let output = Command::new("java")
            .args([
                "-jar", jar_path,
                "--text",
                "--encoding=UTF-8",
            ])
            .arg(&file_str)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("CreateProcess error=2") || stderr.contains("not found") {
                return Err(anyhow::anyhow!("Java is not installed or not in PATH"));
            }
            return Err(anyhow::anyhow!("Tika JAR failed: {}", stderr));
        }

        // Handle potential non-UTF8 output
        let text = String::from_utf8_lossy(&output.stdout).to_string();

        if text.trim().is_empty() {
            return Err(anyhow::anyhow!("Tika JAR returned empty text"));
        }

        Ok(text)
    }

    /// Blocking extraction via JAR - returns full ExtractedContent
    pub fn extract_via_jar_blocking(
        &self,
        file_path: &Path,
        jar_path: &str,
        mime: &str,
    ) -> anyhow::Result<crate::parser::extractor::ExtractedContent> {
        let text = self.extract_via_jar(file_path, jar_path)?;
        Ok(crate::parser::extractor::ExtractedContent {
            text,
            mime_type: mime.to_string(),
            used_tika: true,
        })
    }
}

/// Quick check: is Java available on the system?
pub fn check_java_available() -> bool {
    Command::new("java")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Try to find tika-server or tika-app JAR in common locations
pub fn find_tika_jar() -> Option<String> {
    let candidates = vec![
        "tika-app.jar",
        "tika-server.jar",
        "tika-server-standard.jar",
        "../tika-app.jar",
        "./tools/tika-app.jar",
        "./lib/tika-app.jar",
    ];

    for candidate in candidates {
        if Path::new(candidate).exists() {
            return Some(candidate.to_string());
        }
    }

    None
}
