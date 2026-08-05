import React, { useState, useRef, useEffect, useCallback } from 'react';
import type { SearchMode, SortBy } from '../types';
import { DirectoryPicker } from './DirectoryPicker';

const HISTORY_KEY = 'anywords_history';
const BOOKMARKS_KEY = 'anywords_bookmarks';
const MAX_HISTORY = 20;

const loadList = (key: string): string[] => {
  try {
    const raw = localStorage.getItem(key);
    const arr = raw ? JSON.parse(raw) : [];
    return Array.isArray(arr) ? arr.filter((s) => typeof s === 'string') : [];
  } catch {
    return [];
  }
};

const saveList = (key: string, list: string[]) => {
  try {
    localStorage.setItem(key, JSON.stringify(list));
  } catch {
    // storage full / unavailable — ignore
  }
};

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
  const [showDropdown, setShowDropdown] = useState(false);
  const [history, setHistory] = useState<string[]>(() => loadList(HISTORY_KEY));
  const [bookmarks, setBookmarks] = useState<string[]>(() => loadList(BOOKMARKS_KEY));
  const inputRef = useRef<HTMLInputElement>(null);
  const blurTimer = useRef<number | null>(null);

  // Record a query into history
  const recordHistory = useCallback((term: string) => {
    const t = term.trim();
    if (!t) return;
    setHistory((prev) => {
      const next = [t, ...prev.filter((h) => h !== t)].slice(0, MAX_HISTORY);
      saveList(HISTORY_KEY, next);
      return next;
    });
  }, []);

  const triggerSearch = useCallback(() => {
    recordHistory(q);
    onSearch();
  }, [q, onSearch, recordHistory]);

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

  const toggleBookmark = () => {
    const t = q.trim();
    if (!t) return;
    setBookmarks((prev) => {
      const next = prev.includes(t)
        ? prev.filter((b) => b !== t)
        : [t, ...prev];
      saveList(BOOKMARKS_KEY, next);
      return next;
    });
  };

  const removeHistoryItem = (item: string) => {
    setHistory((prev) => {
      const next = prev.filter((h) => h !== item);
      saveList(HISTORY_KEY, next);
      return next;
    });
  };

  const clearHistory = () => {
    setHistory([]);
    saveList(HISTORY_KEY, []);
  };

  const pickTerm = (term: string) => {
    onQueryChange(term);
    recordHistory(term);
    setShowDropdown(false);
    // Search with the new term right away
    setTimeout(() => onSearch(), 0);
  };

  const isBookmarked = bookmarks.includes(q.trim());
  const hasDropdownContent = bookmarks.length > 0 || history.length > 0;

  return (
    <div className="space-y-2">
      <div className="flex gap-2 items-center">
        <div className="relative flex-1 flex gap-1 bg-[var(--card)] border border-[var(--border)] rounded-2xl shadow-sm p-1.5 focus-within:ring-2 focus-within:ring-[var(--accent)] transition-all">
          <input
            ref={inputRef}
            type="text"
            value={q}
            onChange={(e) => onQueryChange(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && triggerSearch()}
            onFocus={() => {
              if (blurTimer.current) window.clearTimeout(blurTimer.current);
              setShowDropdown(true);
            }}
            onBlur={() => {
              blurTimer.current = window.setTimeout(() => setShowDropdown(false), 150);
            }}
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
            onClick={triggerSearch}
            disabled={loading}
            className="bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white px-6 py-2.5 rounded-xl font-semibold text-sm transition-all disabled:opacity-50"
          >
            {loading ? '搜索中...' : '搜索'}
          </button>

          {/* History / bookmarks dropdown */}
          {showDropdown && hasDropdownContent && (
            <div className="absolute left-0 right-0 top-full mt-2 bg-[var(--card)] border border-[var(--border)] rounded-2xl shadow-xl z-40 max-h-72 overflow-y-auto p-2">
              {bookmarks.length > 0 && (
                <div className="mb-2">
                  <div className="text-xs font-semibold text-[var(--text-secondary)] px-3 py-1">
                    ⭐ 书签
                  </div>
                  {bookmarks.map((b) => (
                    <button
                      key={b}
                      onMouseDown={(e) => { e.preventDefault(); pickTerm(b); }}
                      className="w-full text-left px-3 py-2 rounded-lg text-sm text-[var(--text)] hover:bg-[var(--bg-secondary)] transition-colors flex items-center gap-2"
                    >
                      <span>⭐</span>
                      <span className="truncate">{b}</span>
                    </button>
                  ))}
                </div>
              )}
              {history.length > 0 && (
                <div>
                  <div className="flex items-center justify-between px-3 py-1">
                    <span className="text-xs font-semibold text-[var(--text-secondary)]">
                      🕘 最近搜索
                    </span>
                    <button
                      onMouseDown={(e) => { e.preventDefault(); clearHistory(); }}
                      className="text-xs text-[var(--text-secondary)] hover:text-[var(--danger)]"
                    >
                      清空
                    </button>
                  </div>
                  {history.map((h) => (
                    <div
                      key={h}
                      className="w-full flex items-center gap-2 px-3 py-2 rounded-lg hover:bg-[var(--bg-secondary)] transition-colors group"
                    >
                      <button
                        onMouseDown={(e) => { e.preventDefault(); pickTerm(h); }}
                        className="flex-1 min-w-0 text-left text-sm text-[var(--text)] flex items-center gap-2"
                      >
                        <span className="text-[var(--text-secondary)]">🕘</span>
                        <span className="truncate">{h}</span>
                      </button>
                      <button
                        onMouseDown={(e) => { e.preventDefault(); removeHistoryItem(h); }}
                        className="text-xs text-[var(--text-secondary)] hover:text-[var(--danger)] opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
                        title="删除"
                      >
                        ✕
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>

        {/* Bookmark toggle */}
        <button
          onClick={toggleBookmark}
          disabled={!q.trim()}
          className={`text-lg px-1.5 transition-colors disabled:opacity-30 ${
            isBookmarked ? 'text-amber-400' : 'text-[var(--text-secondary)] hover:text-amber-400'
          }`}
          title={isBookmarked ? '取消书签' : '收藏当前搜索词'}
        >
          {isBookmarked ? '★' : '☆'}
        </button>

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
            <DirectoryPicker
              value={pathFilter}
              onChange={onPathFilterChange}
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
