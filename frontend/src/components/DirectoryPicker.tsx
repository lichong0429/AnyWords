import React, { useState, useEffect, useCallback } from 'react';

interface DirEntry {
  name: string;
  path: string;
  has_children: boolean;
}

interface BrowseResponse {
  path: string;
  parent: string | null;
  directories: DirEntry[];
}

interface DirectoryPickerProps {
  value: string;
  onChange: (path: string) => void;
  label?: string;
  autoSelect?: boolean; // show a per-directory "选定" shortcut button
}

export const DirectoryPicker: React.FC<DirectoryPickerProps> = ({
  value,
  onChange,
  label = '浏览...',
  autoSelect = false,
}) => {
  const [open, setOpen] = useState(false);
  const [currentPath, setCurrentPath] = useState('');
  const [entries, setEntries] = useState<DirEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [roots, setRoots] = useState<DirEntry[]>([]);
  const [rootsLoading, setRootsLoading] = useState(false);
  const [pathInput, setPathInput] = useState('');

  // Fetch roots on first open
  const loadRoots = useCallback(async () => {
    setRootsLoading(true);
    try {
      const res = await fetch('/api/roots');
      if (res.ok) {
        const data: DirEntry[] = await res.json();
        setRoots(data);
      }
    } catch {
      // silent
    } finally {
      setRootsLoading(false);
    }
  }, []);

  useEffect(() => {
    if (open && roots.length === 0 && !rootsLoading) {
      loadRoots();
    }
  }, [open, roots.length, rootsLoading, loadRoots]);

  // Browse a directory
  const browse = useCallback(async (path: string) => {
    if (!path) {
      // Root view: show drives / common folders
      setCurrentPath('');
      setEntries([]);
      setError('');
      setPathInput('');
      return;
    }
    setLoading(true);
    setError('');
    try {
      const res = await fetch(
        `/api/browse?path=${encodeURIComponent(path)}`
      );
      if (res.ok) {
        const data: BrowseResponse = await res.json();
        setCurrentPath(data.path || path);
        setEntries(data.directories);
        setPathInput(data.path || path);
      } else {
        const text = await res.text();
        setEntries([]);
        setError(text || `无法访问: ${path}`);
      }
    } catch {
      setEntries([]);
      setError(`无法访问: ${path}`);
    } finally {
      setLoading(false);
    }
  }, []);

  // Open the picker, browsing the current value when it is an absolute path
  const handleOpen = () => {
    setOpen(true);
    setError('');
    const trimmed = (value || '').trim();
    // Only pre-browse values that look like real paths; a partial keyword
    // filter (e.g. "Documents") would just produce a misleading empty view.
    const looksLikePath = /^[A-Za-z]:[\\/]/.test(trimmed) || trimmed.startsWith('/') || trimmed.startsWith('\\\\');
    if (looksLikePath) {
      browse(trimmed);
    } else {
      browse('');
    }
  };

  // Navigate to a directory
  const handleNavigate = (path: string) => {
    browse(path);
  };

  // Navigate to parent
  const handleUp = () => {
    const parts = currentPath.replace(/\\/g, '/').split('/');
    if (parts.length > 1) {
      parts.pop();
      const parent = parts.join('\\');
      if (parent.endsWith(':')) {
        browse(parent + '\\');
      } else if (parent) {
        browse(parent);
      } else {
        browse('');
      }
    } else {
      browse('');
    }
  };

  // Select directory
  const handleSelect = (path?: string) => {
    const selected = path || currentPath;
    if (!selected) return;
    onChange(selected);
    setOpen(false);
  };

  // Jump to a manually typed path
  const handleJump = () => {
    const p = pathInput.trim().replace(/^["']+|["']+$/g, '');
    if (p) browse(p);
  };

  // Breadcrumb segments
  const breadcrumbs = currentPath
    ? currentPath
        .replace(/\\$/, '')
        .split('\\')
        .filter(Boolean)
    : [];

  return (
    <>
      <button
        onClick={handleOpen}
        className="text-xs text-[var(--accent)] hover:text-[var(--accent-hover)] font-medium px-2 py-1 rounded-md hover:bg-[var(--bg-secondary)] transition-colors whitespace-nowrap"
        type="button"
      >
        {label}
      </button>

      {open && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm"
          onClick={(e) => {
            if (e.target === e.currentTarget) setOpen(false);
          }}
        >
          <div
            className="bg-[var(--card)] border border-[var(--border)] rounded-2xl shadow-2xl w-[520px] max-h-[70vh] flex flex-col overflow-hidden"
            style={{ boxShadow: '0 8px 40px rgba(0,0,0,0.2)' }}
          >
            {/* Header */}
            <div className="flex items-center justify-between px-5 py-3 border-b border-[var(--border)]">
              <h3 className="font-semibold text-[var(--text)]">选择目录</h3>
              <button
                onClick={() => setOpen(false)}
                className="text-[var(--text-secondary)] hover:text-[var(--text)] text-lg leading-none px-1"
              >
                ✕
              </button>
            </div>

            {/* Direct path input */}
            <div className="px-5 py-2 flex gap-2 border-b border-[var(--border)]">
              <input
                type="text"
                value={pathInput}
                onChange={(e) => setPathInput(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleJump()}
                placeholder="直接输入路径, 如 E:\Documents"
                className="flex-1 px-3 py-1.5 bg-[var(--bg-secondary)] border border-[var(--border)] rounded-lg text-sm text-[var(--text)] outline-none focus:ring-1 focus:ring-[var(--accent)]"
              />
              <button
                onClick={handleJump}
                className="px-3 py-1.5 text-sm rounded-lg bg-[var(--bg-secondary)] border border-[var(--border)] text-[var(--text)] hover:bg-[var(--bg)] transition-colors"
              >
                跳转
              </button>
            </div>

            {/* Breadcrumbs */}
            <div className="px-5 py-2 flex items-center gap-1 text-sm overflow-x-auto whitespace-nowrap border-b border-[var(--border)] bg-[var(--bg-secondary)]">
              <button
                onClick={() => browse('')}
                className="text-[var(--accent)] hover:underline shrink-0"
              >
                🖥 此电脑
              </button>
              {breadcrumbs.map((seg, i) => {
                const pathToHere = breadcrumbs.slice(0, i + 1).join('\\') + '\\';
                return (
                  <React.Fragment key={i}>
                    <span className="text-[var(--text-secondary)] shrink-0">
                      ›
                    </span>
                    <button
                      onClick={() => browse(pathToHere)}
                      className={`shrink-0 max-w-[120px] truncate hover:underline ${
                        i === breadcrumbs.length - 1
                          ? 'text-[var(--text)] font-medium'
                          : 'text-[var(--accent)]'
                      }`}
                      title={seg}
                    >
                      {seg}
                    </button>
                  </React.Fragment>
                );
              })}
            </div>

            {/* Content area */}
            <div className="flex-1 overflow-y-auto p-2 min-h-[200px]">
              {/* Parent directory button */}
              {currentPath && (
                <button
                  onClick={handleUp}
                  className="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg hover:bg-[var(--bg-secondary)] transition-colors text-sm text-[var(--text-secondary)]"
                >
                  <span className="text-lg">📁</span>
                  <span>..</span>
                </button>
              )}

              {/* Error message */}
              {error && (
                <div className="mx-2 mb-2 px-3 py-2 rounded-lg text-sm text-[var(--danger)] bg-[var(--bg-secondary)]">
                  ⚠️ {error}
                </div>
              )}

              {/* Root view (no path selected or showing roots) */}
              {!currentPath && (
                <>
                  {rootsLoading ? (
                    <div className="flex items-center justify-center py-8">
                      <div className="animate-spin rounded-full h-6 w-6 border-2 border-[var(--accent)] border-t-transparent" />
                    </div>
                  ) : (
                    roots.map((entry) => (
                      <button
                        key={entry.path}
                        onClick={() => handleNavigate(entry.path)}
                        className="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg hover:bg-[var(--bg-secondary)] transition-colors text-sm text-left"
                      >
                        <span className="text-lg">
                          {entry.name.startsWith('🗂') ? '📂' : '💾'}
                        </span>
                        <span className="text-[var(--text)] font-medium">
                          {entry.name.replace(/^🗂\s*/, '')}
                        </span>
                        <span className="text-[var(--text-secondary)] text-xs ml-auto">
                          {entry.path}
                        </span>
                      </button>
                    ))
                  )}
                </>
              )}

              {/* Directory entries */}
              {currentPath && loading ? (
                <div className="flex items-center justify-center py-8">
                  <div className="animate-spin rounded-full h-6 w-6 border-2 border-[var(--accent)] border-t-transparent" />
                </div>
              ) : currentPath && entries.length === 0 && !error ? (
                <div className="text-center py-8 text-[var(--text-secondary)] text-sm">
                  此目录没有子文件夹
                </div>
              ) : currentPath ? (
                entries.map((entry) => (
                  <div
                    key={entry.path}
                    className="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg hover:bg-[var(--bg-secondary)] transition-colors text-sm text-left group"
                  >
                    <button
                      onClick={() => handleNavigate(entry.path)}
                      className="flex items-center gap-3 flex-1 min-w-0 text-left"
                    >
                      <span className="text-lg">📁</span>
                      <span className="text-[var(--text)] truncate">{entry.name}</span>
                      {entry.has_children && (
                        <span className="ml-auto text-[var(--text-secondary)] text-xs opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
                          展开 ›
                        </span>
                      )}
                    </button>
                    {autoSelect && (
                      <button
                        onClick={() => handleSelect(entry.path)}
                        className="shrink-0 text-xs text-[var(--accent)] hover:underline px-1"
                      >
                        选定
                      </button>
                    )}
                  </div>
                ))
              ) : null}
            </div>

            {/* Footer */}
            <div className="flex items-center justify-between px-5 py-3 border-t border-[var(--border)] bg-[var(--bg-secondary)]">
              <div className="text-xs text-[var(--text-secondary)] truncate max-w-[320px]">
                {currentPath || '未选择目录'}
              </div>
              <div className="flex gap-2">
                <button
                  onClick={() => setOpen(false)}
                  className="px-4 py-1.5 text-sm rounded-lg border border-[var(--border)] text-[var(--text)] hover:bg-[var(--bg)] transition-colors"
                >
                  取消
                </button>
                <button
                  onClick={() => handleSelect()}
                  disabled={!currentPath}
                  className="px-4 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  选择此目录
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </>
  );
};
