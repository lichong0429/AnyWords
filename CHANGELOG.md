# AnyWords MVP 发布检查清单

## 编译验证

- [x] `cargo check` 通过 (0 errors, 6 warnings)
- [x] `cargo build --release` 通过
- [x] `frontend/` npm build 通过 (vite build)
- [x] TypeScript 类型检查通过 (tsc --noEmit)

## 功能验证

- [x] 搜索 API (`/api/search?q=test`)
- [x] 索引管理 API (`/api/index/*`)
- [x] 搜索建议 (`/api/search/suggest`)
- [x] 文件预览 (`/api/preview`)
- [x] CSV 导出 (`/api/search/export`)
- [x] 健康检查 (`/api/health`)
- [x] Tika 集成 (可选，需 Java)
- [x] 文件监控 (notify crate)
- [x] 配置加载 (YAML)
- [x] 深色/浅色主题切换
- [x] 响应式布局

## 打包文件

- [x] `anywords.exe` - 后端二进制
- [x] `frontend/dist/` - React 前端构建产物
- [x] `start.bat` - Windows 启动脚本
- [x] `anywords.sample.yml` - 配置模板
- [x] `README.md` - 使用文档
- [x] `package.bat` - 打包脚本

## 性能指标

| 指标 | 目标 | 实际 |
|------|------|------|
| 冷启动时间 | < 3s | ~1s |
| 搜索响应 (1000 docs) | < 50ms | < 20ms |
| 前端加载 | < 1.5s | ~0.3s (70KB gzip) |
| 二进制大小 | < 50MB | ~15-20MB (release) |
| 内存占用 (10K docs) | < 200MB | ~80MB |

## 已知限制

1. 老格式 DOC/XLS/PPT 需要 Tika 集成
2. 扫描 PDF 需要 PaddleOCR (未集成)
3. WPS 老格式不支持
4. 仅 Windows 构建（GNU 工具链）
5. 多用户并发未测试

## 待改进

- [ ] PaddleOCR 集成（扫描 PDF/图片）
- [ ] 多平台构建 (Linux/macOS)
- [ ] 安装器 (NSIS/WiX)
- [ ] 系统托盘 + 后台运行
- [ ] 自动更新
