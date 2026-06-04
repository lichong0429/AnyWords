// Text extraction module
// Extracts raw text from various document formats for indexing

use std::fs;
use std::io::Read;
use std::path::Path;

use super::tika::TikaParser;

/// Extracted content from a file
#[derive(Debug, Clone)]
pub struct ExtractedContent {
    pub text: String,
    pub mime_type: String,
    /// Whether Tika was used as the parser
    pub used_tika: bool,
}

/// Format that needs Tika for proper parsing
const TIKA_FORMATS: &[&str] = &[
    // Old Microsoft Office binary formats (OLE2)
    "application/msword",                                  // .doc
    "application/vnd.ms-excel",                            // .xls
    "application/vnd.ms-powerpoint",                       // .ppt
    "application/vnd.ms-outlook",                          // .msg
    "application/vnd.visio",                               // .vsd
    // Rich text
    "application/rtf",
    // Help files
    "application/x-chm",
    "application/vnd.ms-htmlhelp",
    // OpenDocument formats
    "application/vnd.oasis.opendocument.text",
    "application/vnd.oasis.opendocument.spreadsheet",
    "application/vnd.oasis.opendocument.presentation",
    // Ebook formats
    "application/x-mobipocket-ebook",                      // .mobi
    "image/vnd.djvu",                                      // .djvu
    // Apple iWork
    "application/vnd.apple.keynote",
    "application/vnd.apple.pages",
    "application/vnd.apple.numbers",
    // Legacy Office
    "application/vnd.ms-works",
    "application/vnd.wordperfect",
];

/// Detect file type and extract text content (without Tika)
pub fn extract_text(file_path: &Path) -> anyhow::Result<ExtractedContent> {
    extract_text_inner(file_path, None)
}

/// Detect file type and extract text content (with optional Tika)
pub async fn extract_text_with_tika(
    file_path: &Path,
    tika: Option<&TikaParser>,
) -> anyhow::Result<ExtractedContent> {
    extract_text_inner(file_path, tika)
}

/// Internal extraction logic
fn extract_text_inner(
    file_path: &Path,
    tika: Option<&TikaParser>,
) -> anyhow::Result<ExtractedContent> {
    let mime = tree_magic_mini::from_filepath(file_path).unwrap_or("application/octet-stream");

    // Check if this format should use Tika
    if TIKA_FORMATS.contains(&mime) {
        if let Some(tika_parser) = tika {
            if tika_parser.is_available() {
                // We can't call async here, so return a placeholder that will be handled upstream
                return Err(anyhow::anyhow!("NEEDS_TIKA:{}", mime));
            }
        }
        // No Tika available, try basic extraction
        return extract_plain_text(file_path, mime)
            .or_else(|_| extract_binary_strings(file_path, mime));
    }

    let mut result = match mime {
        // Plain text files
        "text/plain" | "text/html" | "text/css" | "text/javascript"
        | "text/xml" | "text/csv" | "application/json"
        | "application/xml" | "text/markdown" => {
            extract_plain_text(file_path, mime)
        }

        // Office documents (DOCX, XLSX, PPTX are ZIP archives)
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        | "application/vnd.openxmlformats-officedocument.presentationml.presentation" => {
            extract_ooxml_text(file_path, mime)
        }

        // PDF
        "application/pdf" => {
            extract_pdf_text(file_path, mime)
        }

        // EPUB (ZIP-based)
        "application/epub+zip" => {
            extract_epub_text(file_path, mime)
        }

        // Default: try plain text, then binary extraction
        _ => {
            extract_plain_text(file_path, mime)
                .or_else(|_| extract_binary_strings(file_path, mime))
        }
    };

    // Mark as not using Tika
    if let Ok(ref mut r) = result {
        r.used_tika = false;
    }

    result
}

/// Sync wrapper: try to use Tika for complex formats, fall back to basic
/// This is used from the API handlers which can make blocking calls
pub fn extract_text_sync(
    file_path: &Path,
    tika: Option<&TikaParser>,
) -> anyhow::Result<ExtractedContent> {
    let mime = tree_magic_mini::from_filepath(file_path).unwrap_or("application/octet-stream");

    if TIKA_FORMATS.contains(&mime) {
        if let Some(tika_parser) = tika {
            if tika_parser.is_available() {
                // Try JAR mode synchronously
                if let Some(ref jar_path) = tika_parser.config.jar_path {
                    return tika_parser.extract_via_jar_blocking(file_path, jar_path, mime);
                }
            }
        }
        // Fallback
        return extract_plain_text(file_path, mime)
            .or_else(|_| extract_binary_strings(file_path, mime));
    }

    extract_text(file_path)
}

/// Extract text from plain text files with encoding detection
fn extract_plain_text(file_path: &Path, mime: &str) -> anyhow::Result<ExtractedContent> {
    let raw_bytes = fs::read(file_path)?;

    // Skip binary files that are too large
    if raw_bytes.len() > 50 * 1024 * 1024 {
        // 50MB limit
        return Ok(ExtractedContent {
            text: format!("[File too large: {} MB]", raw_bytes.len() / 1024 / 1024),
            mime_type: mime.to_string(),
            used_tika: false,
        });
    }

    // Detect encoding
    let text = detect_and_decode(&raw_bytes);

    Ok(ExtractedContent {
        text,
        mime_type: mime.to_string(),
        used_tika: false,
    })
}

/// Extract text from OOXML files (DOCX, XLSX, PPTX)
fn extract_ooxml_text(file_path: &Path, mime: &str) -> anyhow::Result<ExtractedContent> {
    let file = fs::File::open(file_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut text = String::new();

    // For DOCX: extract from word/document.xml
    // For XLSX: extract from xl/sharedStrings.xml and xl/worksheets/*.xml
    // For PPTX: extract from ppt/slides/slide*.xml

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_lowercase();

        if name.ends_with(".xml") || name.ends_with(".rels") {
            let mut content = String::new();
            if entry.read_to_string(&mut content).is_ok() {
                text.push_str(&strip_xml_tags(&content));
                text.push('\n');
            }
        }
    }

    if text.is_empty() {
        text = "[No extractable text found in OOXML document]".to_string();
    }

    Ok(ExtractedContent {
        text,
        mime_type: mime.to_string(),
        used_tika: false,
    })
}

/// Extract text from PDF (basic - extracts embedded text)
fn extract_pdf_text(file_path: &Path, mime: &str) -> anyhow::Result<ExtractedContent> {
    let raw_bytes = fs::read(file_path)?;

    // Simple PDF text extraction: look for text between BT/ET markers
    let content = String::from_utf8_lossy(&raw_bytes);
    let mut text = String::new();

    // Parse PDF text blocks (BT ... ET)
    let mut in_text_block = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "BT" {
            in_text_block = true;
            continue;
        }
        if trimmed == "ET" {
            in_text_block = false;
            continue;
        }
        if in_text_block {
            // Extract text from Tj, TJ, ' operators
            if let Some(tj_text) = extract_pdf_tj(trimmed) {
                text.push_str(&tj_text);
            }
        }
    }

    if text.trim().is_empty() {
        text = "[PDF text extraction limited - no embedded text found. OCR may be needed for scanned PDFs]"
            .to_string();
    }

    Ok(ExtractedContent {
        text,
        mime_type: mime.to_string(),
        used_tika: false,
    })
}

/// Extract text from EPUB files
fn extract_epub_text(file_path: &Path, mime: &str) -> anyhow::Result<ExtractedContent> {
    let file = fs::File::open(file_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut text = String::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_lowercase();

        if name.ends_with(".html") || name.ends_with(".xhtml") || name.ends_with(".htm") {
            let mut content = String::new();
            if entry.read_to_string(&mut content).is_ok() {
                text.push_str(&strip_html_tags(&content));
                text.push('\n');
            }
        }
    }

    if text.is_empty() {
        text = "[No text content found in EPUB]".to_string();
    }

    Ok(ExtractedContent {
        text,
        mime_type: mime.to_string(),
        used_tika: false,
    })
}

/// Extract readable strings from binary files
fn extract_binary_strings(file_path: &Path, mime: &str) -> anyhow::Result<ExtractedContent> {
    let raw_bytes = fs::read(file_path)?;

    // Extract ASCII/UTF-8 strings of length >= 4
    let mut text = String::new();
    let mut current = String::new();

    for &byte in &raw_bytes {
        if byte.is_ascii_graphic() || byte == b' ' {
            current.push(byte as char);
        } else {
            if current.len() >= 4 {
                text.push_str(&current);
                text.push('\n');
            }
            current.clear();
        }
    }

    if text.is_empty() {
        text = format!("[Binary file, type: {}]", mime);
    }

    Ok(ExtractedContent {
        text,
        mime_type: mime.to_string(),
        used_tika: false,
    })
}

// --- Utility functions ---

/// Detect encoding and decode to UTF-8 string
fn detect_and_decode(raw_bytes: &[u8]) -> String {
    // Check BOM
    if raw_bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(&raw_bytes[3..]).to_string();
    }
    if raw_bytes.starts_with(&[0xFF, 0xFE]) {
        // UTF-16 LE
        let utf16: Vec<u16> = raw_bytes[2..]
            .chunks(2)
            .filter(|c| c.len() == 2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&utf16);
    }
    if raw_bytes.starts_with(&[0xFE, 0xFF]) {
        // UTF-16 BE
        let utf16: Vec<u16> = raw_bytes[2..]
            .chunks(2)
            .filter(|c| c.len() == 2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&utf16);
    }

    // Try to decode with encoding_rs (detect CJK encodings)
    let cow = encoding_rs::Encoding::for_bom(&[]).map_or(
        std::borrow::Cow::Borrowed(""),
        |enc| enc.0.decode_without_bom_handling(raw_bytes).0,
    );

    if !cow.is_empty() {
        return cow.into_owned();
    }

    // Fallback to lossy UTF-8
    String::from_utf8_lossy(raw_bytes).to_string()
}

/// Strip XML/HTML tags, keeping text content
fn strip_xml_tags(input: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    result
}

/// Strip HTML tags, preserving text
fn strip_html_tags(input: &str) -> String {
    // Simple approach: remove everything between < and >
    let mut result = String::new();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let mut tag_buf = String::new();

    for ch in input.chars() {
        match ch {
            '<' => {
                in_tag = true;
                tag_buf.clear();
            }
            '>' if in_tag => {
                in_tag = false;
                let tag_lower = tag_buf.to_lowercase();
                if tag_lower.starts_with("script") {
                    in_script = true;
                } else if tag_lower == "/script" {
                    in_script = false;
                } else if tag_lower.starts_with("style") {
                    in_style = true;
                } else if tag_lower == "/style" {
                    in_style = false;
                }
                if tag_lower == "br" || tag_lower == "br/" || tag_lower == "p" || tag_lower == "/p" {
                    result.push('\n');
                }
            }
            _ if in_tag => {
                tag_buf.push(ch.to_ascii_lowercase());
            }
            _ if !in_script && !in_style => {
                result.push(ch);
            }
            _ => {}
        }
    }

    // Decode common HTML entities
    result = result.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ");

    result
}

/// Extract text from PDF TJ operator
fn extract_pdf_tj(line: &str) -> Option<String> {
    // Handle "Tj" operator: (text) Tj
    if line.contains("Tj") {
        if let Some(start) = line.find('(') {
            let mut depth = 0;
            let mut text = String::new();
            let bytes = line[start..].as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                match bytes[i] {
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    b'\\' if i + 1 < bytes.len() => {
                        i += 1;
                        match bytes[i] {
                            b'n' => text.push('\n'),
                            b'r' => text.push('\r'),
                            b't' => text.push('\t'),
                            b'\\' => text.push('\\'),
                            b'(' => text.push('('),
                            b')' => text.push(')'),
                            _ => {}
                        }
                        i += 1;
                        continue;
                    }
                    _ => {
                        if depth > 0 {
                            text.push(bytes[i] as char);
                        }
                    }
                }
                i += 1;
            }
            if !text.is_empty() {
                return Some(text);
            }
        }
    }

    // Handle "TJ" operator: [(text) num (text)] TJ
    if line.contains("TJ") && line.contains('(') {
        let mut text = String::new();
        let mut in_paren = false;
        for ch in line.chars() {
            match ch {
                '(' => in_paren = true,
                ')' => in_paren = false,
                _ if in_paren => text.push(ch),
                _ => {}
            }
        }
        if !text.is_empty() {
            return Some(text);
        }
    }

    None
}
