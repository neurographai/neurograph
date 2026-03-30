// ============================================================
// NeuroGraph — SearchBar Component
// Fuzzy search across entities and relationships
// ============================================================

import { useState, useMemo, useRef, useEffect } from 'react';
import type { NeuroGraphData, SearchResult } from '../types/graph';
import { getEntityColor } from '../graph/palette';

interface SearchBarProps {
  data: NeuroGraphData;
  onSelect: (result: SearchResult) => void;
  onHighlight: (nodeIds: string[]) => void;
}

export default function SearchBar({ data, onSelect, onHighlight }: SearchBarProps) {
  const [query, setQuery] = useState('');
  const [isOpen, setIsOpen] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const wrapperRef = useRef<HTMLDivElement>(null);

  // Close on outside click
  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (wrapperRef.current && !wrapperRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClick);
    return () => document.removeEventListener('mousedown', handleClick);
  }, []);

  // Keyboard shortcut: Ctrl+K / Cmd+K
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
        e.preventDefault();
        inputRef.current?.focus();
        setIsOpen(true);
      }
      if (e.key === 'Escape') {
        setIsOpen(false);
        inputRef.current?.blur();
      }
    }
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, []);

  // Fuzzy search
  const results = useMemo<SearchResult[]>(() => {
    if (!query.trim()) return [];
    const q = query.toLowerCase();

    const entityResults: SearchResult[] = data.entities
      .filter(
        (e) =>
          e.name.toLowerCase().includes(q) ||
          e.entity_type.toLowerCase().includes(q) ||
          e.summary.toLowerCase().includes(q) ||
          e.labels.some((l) => l.toLowerCase().includes(q))
      )
      .map((e) => {
        let score = 0;
        if (e.name.toLowerCase().startsWith(q)) score += 3;
        else if (e.name.toLowerCase().includes(q)) score += 2;
        if (e.entity_type.toLowerCase().includes(q)) score += 1;
        score += e.importance_score;
        return {
          id: e.id,
          name: e.name,
          type: 'entity',
          entityType: e.entity_type,
          score,
        };
      })
      .sort((a, b) => b.score - a.score)
      .slice(0, 8);

    const relResults: SearchResult[] = data.relationships
      .filter(
        (r) =>
          r.fact.toLowerCase().includes(q) ||
          r.relationship_type.toLowerCase().includes(q)
      )
      .map((r) => ({
        id: r.id,
        name: r.fact.length > 60 ? r.fact.slice(0, 60) + '…' : r.fact,
        type: 'relationship',
        entityType: r.relationship_type,
        score: r.weight,
      }))
      .slice(0, 4);

    return [...entityResults, ...relResults];
  }, [query, data]);

  // Highlight matching nodes
  useEffect(() => {
    if (query.trim()) {
      const nodeIds = results.filter((r) => r.type === 'entity').map((r) => r.id);
      onHighlight(nodeIds);
    } else {
      onHighlight([]);
    }
  }, [results, query, onHighlight]);

  return (
    <div className="ng-search-wrapper" ref={wrapperRef} id="search-bar">
      <div className={`search-input-container ${isOpen ? 'focused' : ''}`}>
        <svg
          className="search-icon"
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
        >
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>
        <input
          ref={inputRef}
          type="text"
          className="search-input"
          placeholder="Search entities, relationships…"
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setIsOpen(true);
          }}
          onFocus={() => setIsOpen(true)}
          id="search-input"
        />
        <kbd className="search-shortcut">⌘K</kbd>
      </div>

      {isOpen && results.length > 0 && (
        <div className="search-results glass-card animate-fadeInScale" id="search-results">
          {results.map((result) => {
            const color = result.entityType ? getEntityColor(result.entityType) : null;
            return (
              <button
                key={result.id}
                className="search-result-item"
                onClick={() => {
                  onSelect(result);
                  setIsOpen(false);
                  setQuery('');
                }}
              >
                {color && (
                  <span
                    className="search-result-dot"
                    style={{ background: color.fill }}
                  />
                )}
                <div className="search-result-content">
                  <span className="search-result-name">{result.name}</span>
                  <span className="search-result-type">
                    {result.entityType || result.type}
                  </span>
                </div>
                <span className="search-result-score mono">
                  {result.score.toFixed(1)}
                </span>
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
