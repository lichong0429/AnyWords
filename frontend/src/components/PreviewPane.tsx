import React, { useState, useEffect, useMemo } from 'react';
import type { SearchResult } from '../types';
import { api } from '../api';

interface PreviewPaneProps {
  result: SearchResult;
  query: string;
  onClose: () => void;
  onOpenFile: (path: string) => void;
  onRevealFile: (path: string) => void;
}

// Escape regex special chars
const escapeRegex = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

export const PreviewPane: React.FC<PreviewPaneProps> = ({
  result, query, onClose, onOpenFile, onRevealFile,
}) => {
  const [content, setContent] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError('');
    setContent('');
    api.preview(result.file_path)
      .then((res) => {
        if (!cancelled) setContent(res.content);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => { cancelled = true; };
  }, [result.file_path]);

  // Highlight query terms in the preview content
  const highlighted = useMemo(() => {
    if (!content) return null;
    const terms = query
      .trim()
      .split(/\s+/)
      .filter((t) => t.length > 0)
      .map(escapeRegex);
    if (terms.length === 0) return content;
    try {
      const re = new RegExp(`(${terms.join('|')})`, 'gi');
      const parts = content.split(re);
      // With a capturing group, matches land on odd indices after split
      return parts.map((part, i) =>
        i % 2 === 1 ? (
          <mark key={i} className="search-highlight">{part}</mark>
        ) : (
          part
        )
      );
    } catch {
      return content;
    }
  }, [content, query]);

  return (
    <div className="hidden lg:flex flex-col w-[46%] shrink-0 sticky top-6 card overflow-hidden max-h-[calc(100vh-3rem)]">
      {/* Header */}
      <div className="flex items-center gap-2 px-4 py-3 border-b border-[var(--border)] bg-[var(--bg-secondary)]">
        <span className="font-semibold text-sm text-[var(--text)] truncate flex-1" title={result.file_path}>
          {result.file_name}
        </span>
        <button
          onClick={() => onOpenFile(result.file_path)}
          className="text-xs px-2 py-1 rounded-md text-[var(--text-secondary)] hover:text-[var(--accent)] hover:bg-[var(--bg)] transition-colors shrink-0"
          title="打开文件"
        >
          📂 打开
        </button>
        <button
          onClick={() => onRevealFile(result.file_path)}
          className="text-xs px-2 py-1 rounded-md text-[var(--text-secondary)] hover:text-[var(--accent)] hover:bg-[var(--bg)] transition-colors shrink-0"
          title="在文件夹中显示"
        >
          📁 位置
        </button>
        <button
          onClick={onClose}
          className="text-[var(--text-secondary)] hover:text-[var(--text)] text-lg leading-none px-1 shrink-0"
          title="关闭预览"
        >
          ✕
        </button>
      </div>
      <div className="text-xs text-[var(--text-secondary)] px-4 py-1.5 truncate border-b border-[var(--border)]" title={result.file_path}>
        {result.file_path}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-4">
        {loading ? (
          <div className="flex items-center justify-center py-12">
            <div className="animate-spin rounded-full h-6 w-6 border-2 border-[var(--accent)] border-t-transparent" />
          </div>
        ) : error ? (
          <div className="text-sm text-[var(--danger)] py-8 text-center">
            ⚠️ 预览失败: {error}
          </div>
        ) : (
          <pre className="text-[13px] leading-relaxed text-[var(--text)] whitespace-pre-wrap break-words font-mono opacity-90">
            {highlighted}
          </pre>
        )}
      </div>
    </div>
  );
};
