import React, { useState, useEffect, useCallback } from 'react';
import type { IndexStats } from '../types';
import { api } from '../api';
import { DirectoryPicker } from './DirectoryPicker';
import { SettingsPanel } from './SettingsPanel';

interface IndexPanelProps {
  stats: IndexStats | null;
  onRefresh: () => void;
}

export const IndexPanel: React.FC<IndexPanelProps> = ({ stats, onRefresh }) => {
  const [open, setOpen] = useState(false);
  const [scanDir, setScanDir] = useState('');
  const [scanning, setScanning] = useState(false);
  const [message, setMessage] = useState('');
  const [watchDirs, setWatchDirs] = useState<string[]>([]);
  const [busyDir, setBusyDir] = useState('');

  const loadWatchDirs = useCallback(async () => {
    try {
      const res = await api.watchDirs();
      setWatchDirs(res.watch_dirs);
    } catch {
      // silent
    }
  }, []);

  useEffect(() => {
    if (open) loadWatchDirs();
  }, [open, loadWatchDirs]);

  const handleScan = async () => {
    if (!scanDir.trim()) return;
    setScanning(true);
    setMessage('');
    try {
      const res = await api.scan(scanDir);
      setMessage(`${res.message} (${res.count ?? 0} files, ${res.errors ?? 0} errors)`);
      onRefresh();
    } catch (e) {
      setMessage(`Error: ${e}`);
    } finally {
      setScanning(false);
    }
  };

  const handleAddWatchDir = async (path: string) => {
    if (!path.trim() || busyDir) return;
    setBusyDir(path);
    setMessage('');
    try {
      const res = await api.addWatchDir(path);
      setMessage(res.message);
      await loadWatchDirs();
      onRefresh();
    } catch (e) {
      setMessage(`Error: ${e}`);
    } finally {
      setBusyDir('');
    }
  };

  const handleRemoveWatchDir = async (path: string) => {
    if (busyDir) return;
    setBusyDir(path);
    setMessage('');
    try {
      const res = await api.removeWatchDir(path);
      setMessage(res.message);
      await loadWatchDirs();
    } catch (e) {
      setMessage(`Error: ${e}`);
    } finally {
      setBusyDir('');
    }
  };

  const handleRebuild = async () => {
    if (!confirm('Clear all index data and rebuild?')) return;
    try {
      const res = await api.rebuild();
      setMessage(res.message);
      onRefresh();
    } catch (e) {
      setMessage(`Error: ${e}`);
    }
  };

  const formatSize = (b: number) => {
    if (!b) return '0 B';
    const u = ['B', 'KB', 'MB', 'GB'];
    let i = 0, s = b;
    while (s >= 1024 && i < u.length - 1) { s /= 1024; i++; }
    return `${s.toFixed(1)} ${u[i]}`;
  };

  return (
    <div>
      <div className="flex items-center justify-between text-xs text-[var(--text-secondary)] py-1">
        <div className="flex gap-4">
          <span>📄 <strong className="text-[var(--text)]">{(stats?.total_docs ?? 0).toLocaleString()}</strong> 个文件</span>
          <span>💾 <strong className="text-[var(--text)]">{formatSize(stats?.index_size_bytes ?? 0)}</strong></span>
          <span>👁 <strong className="text-[var(--text)]">{watchDirs.length}</strong> 个监控目录</span>
        </div>
        <button
          onClick={() => setOpen(!open)}
          className="text-[var(--accent)] hover:underline text-xs"
        >
          ⚙️ 管理
        </button>
      </div>

      {open && (
        <div className="card p-4 mt-1 space-y-4">
          {/* One-off scan */}
          <div>
            <div className="text-xs font-semibold text-[var(--text-secondary)] mb-1.5">
              📂 临时扫描(仅索引一次, 不加入监控)
            </div>
            <div className="flex gap-2 items-center">
              <input
                type="text"
                value={scanDir}
                onChange={(e) => setScanDir(e.target.value)}
                placeholder="输入目录路径, 或点击浏览选择..."
                className="flex-1 px-3 py-1.5 bg-[var(--bg-secondary)] border border-[var(--border)] rounded-lg text-sm text-[var(--text)] outline-none focus:ring-1 focus:ring-[var(--accent)]"
              />
              <DirectoryPicker
                value={scanDir}
                onChange={setScanDir}
              />
              <button
                onClick={handleScan}
                disabled={scanning || !scanDir.trim()}
                className="bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white px-4 py-1.5 rounded-lg text-sm font-medium transition-all disabled:opacity-50"
              >
                {scanning ? '扫描中...' : '📂 扫描'}
              </button>
              <button
                onClick={handleRebuild}
                className="bg-[var(--bg-secondary)] hover:bg-[var(--border)] text-[var(--text)] border border-[var(--border)] px-4 py-1.5 rounded-lg text-sm transition-all"
              >
                🔄 重建
              </button>
            </div>
          </div>

          {/* Watched directories */}
          <div>
            <div className="text-xs font-semibold text-[var(--text-secondary)] mb-1.5">
              👁 监控目录(自动索引 + 实时更新, 保存到 anywords.yml)
            </div>
            {watchDirs.length === 0 ? (
              <p className="text-xs text-[var(--text-secondary)] px-1 py-1">
                暂无监控目录, 点击下方"添加目录"开始。
              </p>
            ) : (
              <div className="space-y-1">
                {watchDirs.map((dir) => (
                  <div
                    key={dir}
                    className="flex items-center gap-2 px-2 py-1.5 rounded-lg bg-[var(--bg-secondary)] text-sm group"
                  >
                    <span className="text-base">📁</span>
                    <span className="flex-1 min-w-0 truncate text-[var(--text)]" title={dir}>
                      {dir}
                    </span>
                    <button
                      onClick={() => handleRemoveWatchDir(dir)}
                      disabled={busyDir === dir}
                      className="text-xs text-[var(--danger)] hover:underline opacity-0 group-hover:opacity-100 transition-opacity disabled:opacity-50"
                    >
                      {busyDir === dir ? '移除中...' : '移除'}
                    </button>
                  </div>
                ))}
              </div>
            )}
            <div className="flex items-center gap-2 mt-2">
              <DirectoryPicker
                value=""
                onChange={handleAddWatchDir}
                label="➕ 添加目录..."
                autoSelect
              />
              <span className="text-xs text-[var(--text-secondary)]">
                添加后自动开始索引并实时监控文件变化
              </span>
            </div>
          </div>

          {/* Full settings */}
          <SettingsPanel onMessage={setMessage} />

          {scanning && (
            <div className="h-1.5 bg-[var(--bg-secondary)] rounded-full overflow-hidden">
              <div className="h-full bg-gradient-to-r from-[var(--accent)] to-blue-400 rounded-full animate-pulse w-1/2" />
            </div>
          )}
          {message && (
            <p className="text-xs text-[var(--text-secondary)] bg-[var(--bg-secondary)] rounded-lg px-3 py-1.5">
              {message}
            </p>
          )}
        </div>
      )}
    </div>
  );
};
