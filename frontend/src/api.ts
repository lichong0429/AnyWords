// API client for DocSeek backend

import type {
  SearchResponse,
  SearchQuery,
  IndexStats,
  IndexProgress,
  IndexOpResponse,
} from './types';

const BASE = '/api';

async function get<T>(path: string, params?: Record<string, string>): Promise<T> {
  const url = new URL(path, window.location.origin);
  if (params) {
    Object.entries(params).forEach(([k, v]) => {
      if (v) url.searchParams.set(k, v);
    });
  }
  const res = await fetch(url);
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

async function post<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export const api = {
  search: (query: SearchQuery): Promise<SearchResponse> =>
    get(`${BASE}/search`, {
      q: query.q,
      limit: String(query.limit ?? 30),
      offset: String(query.offset ?? 0),
      file_type: query.file_type ?? '',
      path_filter: query.path_filter ?? '',
      mode: query.mode ?? 'fulltext',
      sort: query.sort ?? 'relevance',
      size_min: query.size_min ? String(query.size_min) : '',
      size_max: query.size_max ? String(query.size_max) : '',
    }),

  stats: (): Promise<IndexStats> => get(`${BASE}/index/stats`),

  progress: (): Promise<IndexProgress> => get(`${BASE}/index/progress`),

  scan: (directory: string): Promise<IndexOpResponse> =>
    post('/index/scan', { directory, recursive: true }),

  rebuild: (): Promise<IndexOpResponse> => post('/index/rebuild'),

  exportCsv: (query: SearchQuery): string => {
    const params = new URLSearchParams({
      q: query.q,
      limit: '1000',
      mode: query.mode ?? 'fulltext',
    });
    return `${BASE}/search/export?${params}`;
  },
};
