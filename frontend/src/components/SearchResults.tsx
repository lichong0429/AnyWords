import React from 'react';
import type { SearchResult, FacetCounts } from '../types';

// Helper to get file icon emoji
const fileIcon = (ext: string): string => {
  const icons: Record<string, string> = {
    pdf: '📕', doc: '📘', docx: '📘', xls: '📗', xlsx: '📗',
    ppt: '📙', pptx: '📙', txt: '📄', md: '📝', html: '🌐',
    js: '📜', py: '🐍', rs: '🦀', java: '☕', cpp: '⚙️',
    zip: '📦', rar: '📦', epub: '📚', json: '📋', csv: '📊',
  };
  return icons[ext.toLowerCase()] || '📄';
};

const formatSize = (bytes: number): string => {
  if (!bytes) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  let i = 0, s = bytes;
  while (s >= 1024 && i < units.length - 1) { s /= 1024; i++; }
  return `${s.toFixed(1)} ${units[i]}`;
};

interface ResultCardProps {
  result: SearchResult;
  onCopyPath: (path: string) => void;
}

const ResultCard: React.FC<ResultCardProps> = ({ result, onCopyPath }) => {
  const renderHighlight = (text: string) => {
    // Replace [[keyword]] with <mark>
    const parts = text.split(/(\[\[[^\]]+\]\])/);
    return parts.map((part, i) => {
      if (part.startsWith('[[') && part.endsWith(']]')) {
        return (
          <mark key={i} className="search-highlight">
            {part.slice(2, -2)}
          </mark>
        );
      }
      return part;
    });
  };

  return (
    <div
      onClick={() => onCopyPath(result.file_path)}
      className="card p-4 cursor-pointer hover:shadow-lg hover:-translate-y-0.5 transition-all duration-200 group"
    >
      <div className="flex items-center gap-2.5 mb-1">
        <span className="text-xl w-8 h-8 flex items-center justify-center bg-[var(--bg-secondary)] rounded-lg">
          {fileIcon(result.file_ext)}
        </span>
        <span className="font-semibold text-[var(--accent)] group-hover:underline truncate">
          {result.file_name}
        </span>
        <span className="text-xs text-[var(--text-secondary)] bg-[var(--bg-secondary)] px-1.5 py-0.5 rounded">
          {result.file_ext}
        </span>
      </div>
      <div className="text-xs text-[var(--text-secondary)] truncate mb-2">
        {result.file_path}
      </div>
      <div className="text-sm leading-relaxed text-[var(--text)] opacity-90">
        {renderHighlight(result.snippet)}
        {result.highlights?.map((h, i) => (
          <div key={i} className="text-xs mt-1 opacity-60">
            {renderHighlight(h)}
          </div>
        ))}
      </div>
      <div className="flex gap-4 mt-2.5 text-xs text-[var(--text-secondary)]">
        <span>📅 {result.modified || '-'}</span>
        <span>📦 {result.size_formatted || formatSize(result.size_bytes)}</span>
        <span>⭐ {(result.score * 100).toFixed(1)}%</span>
      </div>
    </div>
  );
};

interface FacetsPanelProps {
  facets: FacetCounts;
  onSelectType: (ext: string) => void;
}

const FacetsPanel: React.FC<FacetsPanelProps> = ({ facets, onSelectType }) => (
  <div className="space-y-5">
    {Object.keys(facets.file_types).length > 0 && (
      <div>
        <h4 className="text-xs font-semibold text-[var(--text-secondary)] uppercase tracking-wide mb-2">
          📁 文件类型
        </h4>
        {Object.entries(facets.file_types)
          .sort((a, b) => b[1] - a[1])
          .slice(0, 10)
          .map(([ext, count]) => (
            <div
              key={ext}
              onClick={() => onSelectType(ext)}
              className="flex justify-between items-center py-1 px-1 text-sm text-[var(--text-secondary)] hover:text-[var(--accent)] cursor-pointer rounded transition-colors"
            >
              <span>{fileIcon(ext)} {ext}</span>
              <span className="font-semibold text-xs">{count}</span>
            </div>
          ))}
      </div>
    )}
    {Object.keys(facets.date_ranges).length > 0 && (
      <div>
        <h4 className="text-xs font-semibold text-[var(--text-secondary)] uppercase tracking-wide mb-2">
          📅 时间范围
        </h4>
        {Object.entries(facets.date_ranges).map(([range, count]) => (
          <div
            key={range}
            className="flex justify-between items-center py-1 px-1 text-sm text-[var(--text-secondary)] rounded"
          >
            <span>{range}</span>
            <span className="font-semibold text-xs">{count}</span>
          </div>
        ))}
      </div>
    )}
  </div>
);

interface SearchResultsProps {
  results: SearchResult[];
  total: number;
  timeMs: number;
  page: number;
  totalPages: number;
  facets: FacetCounts | null;
  loading: boolean;
  error: string | null;
  onSelectType: (ext: string) => void;
  onCopyPath: (path: string) => void;
}

export const SearchResults: React.FC<SearchResultsProps> = ({
  results, total, timeMs, page, totalPages, facets, loading, error, onSelectType, onCopyPath,
}) => {
  if (loading) {
    return (
      <div className="space-y-3 mt-4">
        {Array.from({ length: 5 }).map((_, i) => (
          <div key={i} className="card p-4 space-y-3">
            <div className="flex gap-3 items-center">
              <div className="skeleton w-8 h-8 rounded-lg" />
              <div className="skeleton h-4 w-48" />
            </div>
            <div className="skeleton h-3 w-full" />
            <div className="skeleton h-3 w-2/3" />
          </div>
        ))}
      </div>
    );
  }

  if (error) {
    return (
      <div className="text-center py-20 text-[var(--danger)]">
        <div className="text-4xl mb-3">⚠️</div>
        <p>搜索失败: {error}</p>
      </div>
    );
  }

  if (!results || results.length === 0) {
    return (
      <div className="text-center py-20 text-[var(--text-secondary)]">
        <div className="text-5xl mb-4">📭</div>
        <h3 className="text-lg font-semibold mb-1">没有结果</h3>
        <p className="text-sm">尝试不同的关键词或调整过滤条件</p>
      </div>
    );
  }

  return (
    <div className="mt-4 flex gap-6">
      {/* Results column */}
      <div className="flex-1 min-w-0 space-y-3">
        <div className="text-xs text-[var(--text-secondary)] px-1">
          找到约 <strong className="text-[var(--text)]">{total.toLocaleString()}</strong> 个结果
          （用时 <strong className="text-[var(--text)]">{timeMs.toFixed(0)}ms</strong>）
          {totalPages > 1 && (
            <span> · 第 {page}/{totalPages} 页</span>
          )}
        </div>
        {results.map((r, i) => (
          <ResultCard key={i} result={r} onCopyPath={onCopyPath} />
        ))}
      </div>

      {/* Facets sidebar */}
      {facets && (
        <div className="hidden lg:block w-[200px] flex-shrink-0">
          <FacetsPanel facets={facets} onSelectType={onSelectType} />
        </div>
      )}
    </div>
  );
};
