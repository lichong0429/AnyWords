# DocSeek - 本地文件全文搜索引擎

一个基于 Web 的桌面文件全文搜索工具，类似 AnyTXT Searcher。用浏览器访问你的本地文件，搜索文档内容。

## ✨ 特性

- 🔍 **全文搜索** — 支持中文分词、精确短语、正则表达式、通配符
- 📄 **多格式支持** — TXT/PDF/DOCX/XLSX/PPTX/EPUB/HTML 等，可选 Tika 集成扩展更多格式
- ⚡ **高性能** — 基于 Tantivy 搜索引擎 (比 Lucene 快 2x)，毫秒级响应
- 🌐 **Web 界面** — 浏览器访问 localhost:9921，美观的 React 界面，支持深色/浅色主题
- 📊 **分面聚合** — 自动按文件类型、时间范围统计搜索结果
- 🔄 **实时监控** — 自动检测文件变更并增量索引
- 📦 **单文件部署** — 一个 exe 文件即可运行，无需安装

## 🚀 快速开始

### 直接使用（Windows）

1. 双击 `start.bat`
2. 浏览器自动打开 `http://localhost:9921`
3. 点击「⚙️ 管理」→ 输入目录路径 → 点击「📂 扫描」
4. 索引完成后即可搜索

### 命令行启动

```bash
docseek.exe
# 打开 http://localhost:9921
```

### 配置

复制 `docseek.sample.yml` 为 `docseek.yml`，根据需要修改：

```yaml
server:
  port: 9921

watcher:
  watch_dirs:
    - "E:/Documents"
    - "D:/Projects"
```

## 🛠 开发

### 技术栈

| 层 | 技术 |
|---|------|
| 后端 | Rust + axum + Tantivy |
| 前端 | React 19 + TypeScript + Tailwind CSS 4 |
| 搜索引擎 | Tantivy + tantivy-jieba (中文分词) |
| 构建 | Cargo (Rust) + Vite (前端) |

### 构建

```bash
# 前端
cd frontend && npm install && npm run build

# 后端（需要 Rust 1.96+）
cd .. && cargo build --release

# 运行
./target/release/docseek.exe
```

### 开发模式

```bash
# 终端 1：启动后端
cargo run

# 终端 2：启动前端 dev server（带热重载 + API 代理）
cd frontend && npm run dev
# → http://localhost:5173
```

## 📡 API 接口

| 方法 | 路径 | 说明 |
|------|------|------|
| GET/POST | `/api/search` | 全文搜索 |
| GET | `/api/search/suggest` | 搜索建议 |
| GET | `/api/search/export` | 导出 CSV |
| GET | `/api/preview` | 文件内容预览 |
| GET | `/api/index/stats` | 索引统计 |
| GET | `/api/index/progress` | 索引进度 |
| POST | `/api/index/scan` | 扫描目录 |
| POST | `/api/index/add` | 添加文件 |
| POST | `/api/index/remove` | 移除文件 |
| POST | `/api/index/rebuild` | 重建索引 |
| GET | `/api/health` | 健康检查 |

### 搜索查询参数

```
GET /api/search?q=项目计划&mode=fulltext&sort=relevance&limit=30
                 &file_type=pdf&path_filter=投标&size_min=1024&size_max=1048576
```

## 📦 扩展格式支持（可选）

安装 Apache Tika 以获得更多格式支持：

1. 下载 [tika-app.jar](https://tika.apache.org/download.html)
2. 放到 `docseek.exe` 同级目录
3. 编辑 `docseek.yml`：
   ```yaml
   parser:
     tika_jar_path: "./tika-app.jar"
   ```
4. 需要安装 Java 运行环境

## 📝 许可证

MIT License
