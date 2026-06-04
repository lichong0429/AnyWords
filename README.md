# AnyWords

English | [中文](README_zh.md)

A high-performance local document full-text search engine — an **open-source alternative to AnyTXT Searcher**.

AnyTXT is powerful but paid. AnyWords is free, open-source, and built for both humans and **AI Agents**.

## Why AnyWords?

- **Free & Open Source** — No licenses, no restrictions.
- **AI Agent Ready** — Clean REST API designed for programmatic access. Your AI assistant can search your local files via simple HTTP calls.
- **Fast** — Powered by Tantivy (the Rust equivalent of Lucene). Sub-10ms search on tens of thousands of documents.
- **Broad Format Support** — PDF, DOCX, XLSX, PPTX, TXT, MD, HTML, EPUB, RTF, ODT, and more.

## API for AI Agents

The primary use case: let your AI Agent search your local files.

```bash
# Search
curl "http://localhost:9921/api/search?q=meeting+notes&limit=5"

# Index a directory
curl -X POST "http://localhost:9921/api/index/scan" \
  -H "Content-Type: application/json" \
  -d '{"directory": "C:/Users/You/Documents", "recursive": true}'

# Get index stats
curl "http://localhost:9921/api/index/stats"
```

## Quick Start

1. Download from [Releases](https://github.com/lichong0429/AnyWords/releases)
2. Run `docseek.exe` (yes, the binary name will be `anywords.exe` in next release)
3. Open `http://localhost:9921` in your browser

Or build from source:

```bash
cargo build --release
./target/release/anywords.exe
```

## Configuration

Edit `anywords.yml` (auto-created on first run):

```yaml
server:
  host: "0.0.0.0"
  port: 9921

index:
  dir: "~/.anywords/index"
  auto_commit: true

scan:
  max_file_size: 52428800  # 50MB
  max_depth: 20
  follow_symlinks: false
```

## Web UI

A built-in React frontend is served at `http://localhost:9921`.

## Tech Stack

- **Backend**: Rust + Axum + Tantivy
- **Frontend**: React + TypeScript + Vite
- **Supported formats**: PDF, DOCX, XLSX, PPTX, TXT, MD, HTML, EPUB, RTF, ODT

## License

MIT
