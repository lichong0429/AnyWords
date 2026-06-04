import React, { useState } from 'react';
import type { IndexStats } from '../types';
import { api } from '../api';

interface IndexPanelProps {
  stats: IndexStats | null;
  onRefresh: () => void;
}

export const IndexPanel: React.FC<IndexPanelProps> = ({ stats, onRefresh }) => {
  const [open, setOpen] = useState(false);
  const [scanDir, setScanDir] = useState('');
  const [scanning, setScanning] = useState(false);
  const [message, setMessage] = useState('');

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
        </div>
        <button
          onClick={() => setOpen(!open)}
          className="text-[var(--accent)] hover:underline text-xs"
        >
          ⚙️ 管理
        </button>
      </div>

      {open && (
        <div className="card p-4 mt-1 space-y-3">
          <div className="flex gap-2">
            <input
              type="text"
              value={scanDir}
              onChange={(e) => setScanDir(e.target.value)}
              placeholder="输入目录路径..."
              className="flex-1 px-3 py-1.5 bg-[var(--bg-secondary)] border border-[var(--border)] rounded-lg text-sm text-[var(--text)] outline-none focus:ring-1 focus:ring-[var(--accent)]"
            />
            <button
              onClick={handleScan}
              disabled={scanning}
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
