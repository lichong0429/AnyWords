import React, { useState, useRef, useEffect } from 'react';
import type { SearchMode, SortBy } from '../types';

interface SearchBarProps {
  q: string;
  mode: SearchMode;
  sort: SortBy;
  fileType: string;
  pathFilter: string;
  sizeMin: string;
  sizeMax: string;
  onQueryChange: (q: string) => void;
  onModeChange: (mode: SearchMode) => void;
  onSortChange: (sort: SortBy) => void;
  onFileTypeChange: (v: string) => void;
  onPathFilterChange: (v: string) => void;
  onSizeMinChange: (v: string) => void;
  onSizeMaxChange: (v: string) => void;
  onSearch: () => void;
  onExport: () => void;
  loading: boolean;
}

export const SearchBar: React.FC<SearchBarProps> = ({
  q, mode, sort, fileType, pathFilter, sizeMin, sizeMax,
  onQueryChange, onModeChange, onSortChange,
  onFileTypeChange, onPathFilterChange, onSizeMinChange, onSizeMaxChange,
  onSearch, onExport, loading,
}) => {
  const [showFilters, setShowFilters] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  // Ctrl+K to focus
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
        e.preventDefault();
        inputRef.current?.focus();
      }
      if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
        e.preventDefault();
        onExport();
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [onExport]);

  return (
    <div className="space-y-2">
      <div className="flex gap-2 items-center">
        <div className="flex-1 flex gap-1 bg-[var(--card)] border border-[var(--border)] rounded-2xl shadow-sm p-1.5 focus-within:ring-2 focus-within:ring-[var(--accent)] transition-all">
          <input
            ref={inputRef}
            type="text"
            value={q}
            onChange={(e) => onQueryChange(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && onSearch()}
            placeholder="搜索文件内容... (Ctrl+K 聚焦, Ctrl+Enter 导出)"
            className="flex-1 bg-transparent px-4 py-2.5 text-[15px] outline-none text-[var(--text)] placeholder:text-[var(--text-secondary)]"
            autoFocus
          />
          <select
            value={mode}
            onChange={(e) => onModeChange(e.target.value as SearchMode)}
            className="bg-[var(--bg-secondary)] text-[var(--text)] text-sm px-3 py-2 rounded-xl outline-none border-0 cursor-pointer"
          >
            <option value="fulltext">全文</option>
            <option value="phrase">短语</option>
            <option value="regex">正则</option>
            <option value="wildcard">通配</option>
          </select>
          <select
            value={sort}
            onChange={(e) => onSortChange(e.target.value as SortBy)}
            className="bg-[var(--bg-secondary)] text-[var(--text)] text-sm px-3 py-2 rounded-xl outline-none border-0 cursor-pointer"
          >
            <option value="relevance">相关度</option>
            <option value="date">日期</option>
            <option value="size">大小</option>
            <option value="name">文件名</option>
          </select>
          <button
            onClick={onSearch}
            disabled={loading}
            className="bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white px-6 py-2.5 rounded-xl font-semibold text-sm transition-all disabled:opacity-50"
          >
            {loading ? '搜索中...' : '搜索'}
          </button>
        </div>
        <button
          onClick={() => setShowFilters(!showFilters)}
          className="text-[var(--text-secondary)] hover:text-[var(--accent)] text-sm px-2 transition-colors"
          title="高级过滤"
        >
          ⚙️
        </button>
      </div>

      {showFilters && (
        <div className="flex gap-3 items-center flex-wrap text-sm">
          <label className="flex items-center gap-1 text-[var(--text-secondary)]">
            类型:
            <input
              type="text"
              value={fileType}
              onChange={(e) => onFileTypeChange(e.target.value)}
              placeholder="pdf,docx"
              className="w-24 px-2 py-1 bg-[var(--bg-secondary)] border border-[var(--border)] rounded-md text-[var(--text)] outline-none focus:ring-1 focus:ring-[var(--accent)]"
            />
          </label>
          <label className="flex items-center gap-1 text-[var(--text-secondary)]">
            路径:
            <input
              type="text"
              value={pathFilter}
              onChange={(e) => onPathFilterChange(e.target.value)}
              placeholder="包含路径..."
              className="w-36 px-2 py-1 bg-[var(--bg-secondary)] border border-[var(--border)] rounded-md text-[var(--text)] outline-none focus:ring-1 focus:ring-[var(--accent)]"
            />
          </label>
          <span className="text-[var(--text-secondary)]">
            大小:
            <input
              type="number"
              value={sizeMin}
              onChange={(e) => onSizeMinChange(e.target.value)}
              placeholder="最小"
              className="w-20 ml-1 px-2 py-1 bg-[var(--bg-secondary)] border border-[var(--border)] rounded-md text-[var(--text)] outline-none focus:ring-1 focus:ring-[var(--accent)]"
              min="0"
            />
            <span className="mx-1">-</span>
            <input
              type="number"
              value={sizeMax}
              onChange={(e) => onSizeMaxChange(e.target.value)}
              placeholder="最大"
              className="w-20 px-2 py-1 bg-[var(--bg-secondary)] border border-[var(--border)] rounded-md text-[var(--text)] outline-none focus:ring-1 focus:ring-[var(--accent)]"
              min="0"
            />
            <span className="ml-1">KB</span>
          </span>
        </div>
      )}
    </div>
  );
};
