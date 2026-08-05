import React, { useState, useEffect, useMemo } from 'react';
import hljs from 'highlight.js/lib/core';
import javascript from 'highlight.js/lib/languages/javascript';
import typescript from 'highlight.js/lib/languages/typescript';
import python from 'highlight.js/lib/languages/python';
import rust from 'highlight.js/lib/languages/rust';
import java from 'highlight.js/lib/languages/java';
import c from 'highlight.js/lib/languages/c';
import cpp from 'highlight.js/lib/languages/cpp';
import csharp from 'highlight.js/lib/languages/csharp';
import go from 'highlight.js/lib/languages/go';
import ruby from 'highlight.js/lib/languages/ruby';
import php from 'highlight.js/lib/languages/php';
import bash from 'highlight.js/lib/languages/bash';
import sql from 'highlight.js/lib/languages/sql';
import css from 'highlight.js/lib/languages/css';
import xml from 'highlight.js/lib/languages/xml';
import json from 'highlight.js/lib/languages/json';
import yaml from 'highlight.js/lib/languages/yaml';
import markdown from 'highlight.js/lib/languages/markdown';
import ini from 'highlight.js/lib/languages/ini';
import lua from 'highlight.js/lib/languages/lua';
import r from 'highlight.js/lib/languages/r';
import 'highlight.js/styles/github-dark.css';

import type { SearchResult } from '../types';
import { api } from '../api';

// Register languages once
hljs.registerLanguage('javascript', javascript);
hljs.registerLanguage('typescript', typescript);
hljs.registerLanguage('python', python);
hljs.registerLanguage('rust', rust);
hljs.registerLanguage('java', java);
hljs.registerLanguage('c', c);
hljs.registerLanguage('cpp', cpp);
hljs.registerLanguage('csharp', csharp);
hljs.registerLanguage('go', go);
hljs.registerLanguage('ruby', ruby);
hljs.registerLanguage('php', php);
hljs.registerLanguage('bash', bash);
hljs.registerLanguage('sql', sql);
hljs.registerLanguage('css', css);
hljs.registerLanguage('xml', xml);
hljs.registerLanguage('json', json);
hljs.registerLanguage('yaml', yaml);
hljs.registerLanguage('markdown', markdown);
hljs.registerLanguage('ini', ini);
hljs.registerLanguage('lua', lua);
hljs.registerLanguage('r', r);

const IMAGE_EXTS = new Set(['png', 'jpg', 'jpeg', 'gif', 'bmp', 'webp', 'svg', 'ico']);

// Map file extension -> highlight.js language id
const CODE_LANGS: Record<string, string> = {
  js: 'javascript', mjs: 'javascript', jsx: 'javascript',
  ts: 'typescript', tsx: 'typescript',
  py: 'python', rs: 'rust', java: 'java',
  c: 'c', h: 'c', cpp: 'cpp', hpp: 'cpp', cc: 'cpp',
  cs: 'csharp', go: 'go', rb: 'ruby', php: 'php',
  sh: 'bash', bash: 'bash', zsh: 'bash',
  sql: 'sql', css: 'css',
  html: 'xml', htm: 'xml', xml: 'xml', svg: 'xml', vue: 'xml',
  json: 'json', yaml: 'yaml', yml: 'yaml',
  toml: 'ini', ini: 'ini', cfg: 'ini',
  md: 'markdown', lua: 'lua', r: 'r',
};

// Escape regex special chars
const escapeRegex = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

interface PreviewPaneProps {
  result: SearchResult;
  query: string;
  onClose: () => void;
  onOpenFile: (path: string) => void;
  onRevealFile: (path: string) => void;
}

export const PreviewPane: React.FC<PreviewPaneProps> = ({
  result, query, onClose, onOpenFile, onRevealFile,
}) => {
  const [content, setContent] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const ext = result.file_ext.toLowerCase();
  const isImage = IMAGE_EXTS.has(ext);
  const isPdf = ext === 'pdf';
  const codeLang = CODE_LANGS[ext];
  // Images/PDFs render from raw bytes — no text fetch needed
  const needsText = !isImage && !isPdf;

  useEffect(() => {
    if (!needsText) {
      setContent('');
      setError('');
      setLoading(false);
      return;
    }
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
  }, [result.file_path, needsText]);

  // Highlight query terms in plain-text preview
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

  // Syntax-highlighted code preview
  const highlightedCode = useMemo(() => {
    if (!codeLang || !content) return null;
    try {
      return hljs.highlight(content, { language: codeLang, ignoreIllegals: true }).value;
    } catch {
      return null;
    }
  }, [codeLang, content]);

  const rawUrl = api.rawFileUrl(result.file_path);

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
      <div className="flex-1 overflow-auto min-h-[300px]">
        {isImage ? (
          <div className="flex items-center justify-center p-4 h-full">
            <img
              src={rawUrl}
              alt={result.file_name}
              className="max-w-full max-h-full object-contain rounded-lg"
            />
          </div>
        ) : isPdf ? (
          <iframe
            src={rawUrl}
            title={result.file_name}
            className="w-full h-full min-h-[70vh] border-0"
          />
        ) : loading ? (
          <div className="flex items-center justify-center py-12">
            <div className="animate-spin rounded-full h-6 w-6 border-2 border-[var(--accent)] border-t-transparent" />
          </div>
        ) : error ? (
          <div className="text-sm text-[var(--danger)] py-8 text-center px-4">
            ⚠️ 预览失败: {error}
          </div>
        ) : codeLang && highlightedCode ? (
          <pre className="text-[12.5px] leading-relaxed p-4 min-h-full bg-[#0d1117]">
            <code
              className="hljs font-mono whitespace-pre-wrap break-words"
              dangerouslySetInnerHTML={{ __html: highlightedCode }}
            />
          </pre>
        ) : (
          <pre className="text-[13px] leading-relaxed text-[var(--text)] whitespace-pre-wrap break-words font-mono opacity-90 p-4">
            {highlighted}
          </pre>
        )}
      </div>
    </div>
  );
};
