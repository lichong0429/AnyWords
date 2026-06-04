import { useState, useCallback, useEffect } from 'react';
import type { SearchQuery, SearchResponse, SearchMode, SortBy } from './types';
import { api } from './api';

export interface UseSearchReturn {
  query: SearchQuery;
  setQuery: (q: Partial<SearchQuery>) => void;
  results: SearchResponse | null;
  loading: boolean;
  error: string | null;
  search: () => Promise<void>;
}

export function useSearch(): UseSearchReturn {
  const [query, setQueryState] = useState<SearchQuery>({
    q: '',
    limit: 30,
    offset: 0,
    mode: 'fulltext' as SearchMode,
    sort: 'relevance' as SortBy,
  });
  const [results, setResults] = useState<SearchResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const setQuery = useCallback((partial: Partial<SearchQuery>) => {
    setQueryState((prev) => ({ ...prev, ...partial }));
  }, []);

  const search = useCallback(async () => {
    if (!query.q.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const data = await api.search(query);
      setResults(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Search failed');
    } finally {
      setLoading(false);
    }
  }, [query]);

  // Trigger search when query changes (debounced externally)
  useEffect(() => {
    if (query.q.trim()) {
      const timer = setTimeout(search, 300);
      return () => clearTimeout(timer);
    }
  }, [query.q, query.mode, query.sort, query.file_type, query.path_filter]);

  return { query, setQuery, results, loading, error, search };
}
