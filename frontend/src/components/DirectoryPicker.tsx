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
}

export const DirectoryPicker: React.FC<DirectoryPickerProps> = ({
  value,
  onChange,
}) => {
  const [open, setOpen] = useState(false);
  const [currentPath, setCurrentPath] = useState(value || '');
  const [entries, setEntries] = useState<DirEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [roots, setRoots] = useState<DirEntry[]>([]);
  const [rootsLoading, setRootsLoading] = useState(false);

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
    setLoading(true);
    setCurrentPath(path);
    try {
      const res = await fetch(
        `/api/browse?path=${encodeURIComponent(path)}`
      );
      if (res.ok) {
        const data: BrowseResponse = await res.json();
        setEntries(data.directories);
      } else {
        setEntries([]);
      }
    } catch {
      setEntries([]);
    } finally {
      setLoading(false);
    }
  }, []);

  // Open the picker and browse current value or roots
  const handleOpen = () => {
    setOpen(true);
    if (value && value !== currentPath) {
      browse(value);
    }
  };

  // Navigate to a directory
  const handleNavigate = (path: string) => {
    browse(path);
  };

  // Navigate to parent
  const handleUp = () => {
    // Get parent path
    const parts = currentPath.replace(/\\/g, '/').split('/');
    if (parts.length > 1) {
      parts.pop();
      const parent = parts.join('\\');
      if (parent.endsWith(':')) {
        browse(parent + '\\');
      } else {
        browse(parent);
      }
    }
  };

  // Select directory
  const handleSelect = (path?: string) => {
    const selected = path || currentPath;
    onChange(selected);
    setOpen(false);
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
        浏览...
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
              {currentPath && breadcrumbs.length > 1 && (
                <button
                  onClick={handleUp}
                  className="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg hover:bg-[var(--bg-secondary)] transition-colors text-sm text-[var(--text-secondary)]"
                >
                  <span className="text-lg">📁</span>
                  <span>..</span>
                </button>
              )}

              {/* Root view (no path selected or showing roots) */}
              {(!currentPath || currentPath === '') && (
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
              {currentPath && currentPath !== '' && loading ? (
                <div className="flex items-center justify-center py-8">
                  <div className="animate-spin rounded-full h-6 w-6 border-2 border-[var(--accent)] border-t-transparent" />
                </div>
              ) : currentPath && currentPath !== '' && entries.length === 0 ? (
                <div className="text-center py-8 text-[var(--text-secondary)] text-sm">
                  此目录为空
                </div>
              ) : currentPath && currentPath !== '' ? (
                entries.map((entry) => (
                  <button
                    key={entry.path}
                    onClick={() => handleNavigate(entry.path)}
                    className="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg hover:bg-[var(--bg-secondary)] transition-colors text-sm text-left group"
                  >
                    <span className="text-lg">
                      {entry.has_children ? '📁' : '📄'}
                    </span>
                    <span className="text-[var(--text)]">{entry.name}</span>
                    {entry.has_children && (
                      <span className="ml-auto text-[var(--text-secondary)] text-xs opacity-0 group-hover:opacity-100 transition-opacity">
                        展开 ›
                      </span>
                    )}
                  </button>
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
                  className="px-4 py-1.5 text-sm rounded-lg bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white font-medium transition-colors"
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
