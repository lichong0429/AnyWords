# AnyWords

[English](README.md) | 中文

一个高性能本地文档全文搜索引擎 —— **AnyTXT Searcher 的开源替代品**。

AnyTXT 很强大，但要收费。AnyWords 免费、开源，既给人用，也给 **AI Agent** 用。

## 为什么做 AnyWords？

- **免费开源** — 无授权、无限制。
- **AI Agent 友好** — 专为程序化访问设计的 REST API。你的 AI 助手可以通过简单的 HTTP 调用搜索你电脑上的文件。
- **速度快** — 基于 Tantivy（Rust 版的 Lucene）构建，上万文档搜索耗时 <10ms。
- **格式支持广** — PDF、DOCX、XLSX、PPTX、TXT、MD、HTML、EPUB、RTF、ODT 等。

## 给 AI Agent 用的 API

核心使用场景：让你的 AI Agent 搜索本地文件。

```bash
# 搜索
curl "http://localhost:9921/api/search?q=会议记录&limit=5"

# 索引一个目录
curl -X POST "http://localhost:9921/api/index/scan" \
  -H "Content-Type: application/json" \
  -d '{"directory": "C:/Users/You/Documents", "recursive": true}'

# 查看索引状态
curl "http://localhost:9921/api/index/stats"
```

## 快速开始

1. 从 [Releases](https://github.com/lichong0429/AnyWords/releases) 下载
2. 运行 `anywords.exe`
3. 浏览器打开 `http://localhost:9921`

或从源码编译：

```bash
cargo build --release
./target/release/anywords.exe
```

## 配置

首次运行会自动创建 `anywords.yml`：

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

## Web 界面

内置 React 前端，访问 `http://localhost:9921` 即可使用。

## 技术栈

- **后端**：Rust + Axum + Tantivy
- **前端**：React + TypeScript + Vite
- **支持格式**：PDF、DOCX、XLSX、PPTX、TXT、MD、HTML、EPUB、RTF、ODT

## 许可证

MIT
