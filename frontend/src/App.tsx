import React, { useState, useEffect, useCallback } from 'react';
import { SearchBar } from './components/SearchBar';
import { SearchResults } from './components/SearchResults';
import { IndexPanel } from './components/IndexPanel';
import { PreviewPane } from './components/PreviewPane';
import { useSearch } from './hooks';
import { api } from './api';
import type { IndexStats, SearchResult } from './types';

const App: React.FC = () => {
  const { query, setQuery, results, loading, error, search } = useSearch();
  const [stats, setStats] = useState<IndexStats | null>(null);
  const [dark, setDark] = useState(() =>
    window.matchMedia('(prefers-color-scheme: dark)').matches
  );
  const [copied, setCopied] = useState(false);
  const [toast, setToast] = useState('');
  const [preview, setPreview] = useState<SearchResult | null>(null);

  const showToast = useCallback((text: string) => {
    setToast(text);
    setTimeout(() => setToast(''), 2500);
  }, []);

  // Theme management
  useEffect(() => {
    document.documentElement.classList.toggle('dark', dark);
  }, [dark]);

  useEffect(() => {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = (e: MediaQueryListEvent) => setDark(e.matches);
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, []);

  // Stats polling
  const loadStats = useCallback(async () => {
    try {
      setStats(await api.stats());
    } catch { /* silent */ }
  }, []);

  useEffect(() => {
    loadStats();
    const timer = setInterval(loadStats, 10000);
    return () => clearInterval(timer);
  }, [loadStats]);

  const handleSearch = () => search();

  const handleExport = () => {
    window.open(api.exportCsv(query), '_blank');
  };

  const handleSelectType = (ext: string) => {
    setQuery({ file_type: ext });
    search();
  };

  const handleCopyPath = async (path: string) => {
    await navigator.clipboard.writeText(path);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleOpenFile = async (path: string) => {
    try {
      const res = await api.openFile(path);
      if (!res.success) showToast(`⚠️ ${res.message}`);
    } catch (e) {
      showToast(`⚠️ 无法打开文件: ${e}`);
    }
  };

  const handleRevealFile = async (path: string) => {
    try {
      const res = await api.openFile(path, true);
      if (!res.success) showToast(`⚠️ ${res.message}`);
    } catch (e) {
      showToast(`⚠️ 无法打开所在目录: ${e}`);
    }
  };

  const handlePreview = (r: SearchResult) => {
    setPreview((prev) => (prev?.file_path === r.file_path ? prev : r));
  };

  return (
    <div className="min-h-screen">
      <div className={`${preview ? 'max-w-7xl' : 'max-w-5xl'} mx-auto px-5 py-8 transition-all`}>
        {/* Header */}
        <header className="text-center mb-8">
          <div className="flex justify-end mb-2">
            <button
              onClick={() => setDark(!dark)}
              className="text-sm text-[var(--text-secondary)] hover:text-[var(--text)] transition-colors px-3 py-1 rounded-lg"
            >
              {dark ? '☀️ 浅色' : '🌙 深色'}
            </button>
          </div>
          <h1 className="text-3xl font-bold tracking-tight mb-1">
            🔍 AnyWords
          </h1>
          <p className="text-[var(--text-secondary)]">
            本地文件全文搜索引擎
          </p>
        </header>

        {/* Search bar */}
        <SearchBar
          q={query.q}
          mode={query.mode!}
          sort={query.sort!}
          fileType={query.file_type ?? ''}
          pathFilter={query.path_filter ?? ''}
          sizeMin={query.size_min ? String(query.size_min / 1024) : ''}
          sizeMax={query.size_max ? String(query.size_max / 1024) : ''}
          onQueryChange={(v) => setQuery({ q: v })}
          onModeChange={(v) => setQuery({ mode: v })}
          onSortChange={(v) => setQuery({ sort: v })}
          onFileTypeChange={(v) => setQuery({ file_type: v || undefined })}
          onPathFilterChange={(v) => setQuery({ path_filter: v || undefined })}
          onSizeMinChange={(v) => setQuery({ size_min: v ? Number(v) * 1024 : undefined })}
          onSizeMaxChange={(v) => setQuery({ size_max: v ? Number(v) * 1024 : undefined })}
          onSearch={handleSearch}
          onExport={handleExport}
          loading={loading}
        />

        {/* Index panel */}
        <div className="mt-4 px-1">
          <IndexPanel stats={stats} onRefresh={loadStats} />
        </div>

        {/* Results + Preview */}
        <div className="flex gap-6 items-start">
          <div className="flex-1 min-w-0">
            <SearchResults
              results={results?.results ?? []}
              total={results?.total ?? 0}
              timeMs={results?.time_ms ?? 0}
              page={results?.page ?? 1}
              totalPages={results?.total_pages ?? 1}
              facets={results?.facets ?? null}
              loading={loading}
              error={error}
              previewPath={preview?.file_path ?? null}
              onPreview={handlePreview}
              onSelectType={handleSelectType}
              onCopyPath={handleCopyPath}
              onOpenFile={handleOpenFile}
              onRevealFile={handleRevealFile}
            />
          </div>
          {preview && (
            <PreviewPane
              result={preview}
              query={query.q}
              onClose={() => setPreview(null)}
              onOpenFile={handleOpenFile}
              onRevealFile={handleRevealFile}
            />
          )}
        </div>

        {/* Copy notification */}
        {copied && (
          <div className="fixed bottom-6 right-6 bg-[var(--accent)] text-white px-4 py-2 rounded-xl shadow-lg text-sm font-medium animate-bounce">
            路径已复制 ✅
          </div>
        )}

        {/* Toast notification */}
        {toast && (
          <div className="fixed bottom-6 right-6 bg-[var(--card)] border border-[var(--border)] text-[var(--text)] px-4 py-2 rounded-xl shadow-lg text-sm font-medium">
            {toast}
          </div>
        )}
      </div>
    </div>
  );
};

export default App;
