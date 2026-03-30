// ============================================================
// NeuroGraph — Sidebar Component
// Entity detail panel + AI query panel
// ============================================================

import { useState } from 'react';
import type { NeuroGraphData, SelectedElement } from '../types/graph';
import { getEntityColor } from '../graph/palette';

interface SidebarProps {
  data: NeuroGraphData;
  selected: SelectedElement | null;
  onClose: () => void;
}

export default function Sidebar({ data, selected, onClose }: SidebarProps) {
  const [activeTab, setActiveTab] = useState<'details' | 'query'>('details');
  const [queryText, setQueryText] = useState('');
  const [queryResult, setQueryResult] = useState<string | null>(null);
  const [isQuerying, setIsQuerying] = useState(false);

  const handleQuery = () => {
    if (!queryText.trim()) return;
    setIsQuerying(true);
    setQueryResult(null);

    // Simulate AI query response
    setTimeout(() => {
      setQueryResult(
        `Based on the knowledge graph analysis:\n\n${queryText}\n\n` +
        `The graph contains ${data.entities.length} entities and ${data.relationships.length} relationships ` +
        `across ${data.communities.length} communities. ` +
        `Key insights from the connected entities suggest relevant patterns in the data.`
      );
      setIsQuerying(false);
    }, 1500);
  };

  // Find entity data for the selected node
  const selectedEntity = selected?.type === 'node'
    ? data.entities.find((e) => e.id === selected.id)
    : null;

  // Find related relationships for selected entity
  const relatedRelationships = selectedEntity
    ? data.relationships.filter(
        (r) =>
          r.source_entity_id === selectedEntity.id ||
          r.target_entity_id === selectedEntity.id
      )
    : [];

  // Get connected entity names
  const getEntityName = (id: string) =>
    data.entities.find((e) => e.id === id)?.name || id;

  // Find selected edge data
  const selectedEdge = selected?.type === 'edge'
    ? data.relationships.find((r) => r.id === selected.id)
    : null;

  return (
    <div className="ng-sidebar" id="sidebar">
      {/* Tab bar */}
      <div className="sidebar-tabs">
        <button
          className={`sidebar-tab ${activeTab === 'details' ? 'active' : ''}`}
          onClick={() => setActiveTab('details')}
          id="tab-details"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="16" x2="12" y2="12" />
            <line x1="12" y1="8" x2="12.01" y2="8" />
          </svg>
          Details
        </button>
        <button
          className={`sidebar-tab ${activeTab === 'query' ? 'active' : ''}`}
          onClick={() => setActiveTab('query')}
          id="tab-query"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
          </svg>
          AI Query
        </button>
        <button className="sidebar-close icon-btn" onClick={onClose} id="btn-close-sidebar">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </button>
      </div>

      <div className="divider-h" />

      {/* Details Tab */}
      {activeTab === 'details' && (
        <div className="sidebar-content animate-fadeIn" id="sidebar-details">
          {selectedEntity ? (
            <EntityDetail
              entity={selectedEntity}
              relationships={relatedRelationships}
              getEntityName={getEntityName}
            />
          ) : selectedEdge ? (
            <EdgeDetail
              edge={selectedEdge}
              getEntityName={getEntityName}
            />
          ) : (
            <div className="sidebar-empty">
              <div className="sidebar-empty-icon">
                <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1" opacity="0.3">
                  <circle cx="12" cy="12" r="10" />
                  <path d="M8 14s1.5 2 4 2 4-2 4-2" />
                  <line x1="9" y1="9" x2="9.01" y2="9" />
                  <line x1="15" y1="9" x2="15.01" y2="9" />
                </svg>
              </div>
              <p className="sidebar-empty-text">Click on a node or edge to inspect</p>
              <p className="sidebar-empty-hint">Use the search bar to find entities</p>
            </div>
          )}
        </div>
      )}

      {/* AI Query Tab */}
      {activeTab === 'query' && (
        <div className="sidebar-content animate-fadeIn" id="sidebar-query">
          <div className="query-section">
            <h3 className="sidebar-section-title">Ask the Knowledge Graph</h3>
            <p className="sidebar-section-desc">
              Natural language queries powered by NeuroGraph's retrieval engine.
            </p>

            <div className="query-input-wrapper">
              <textarea
                className="query-textarea"
                placeholder="e.g., What are the key technologies behind NeuroGraph?"
                value={queryText}
                onChange={(e) => setQueryText(e.target.value)}
                rows={3}
                id="query-input"
              />
              <button
                className="query-submit"
                onClick={handleQuery}
                disabled={isQuerying || !queryText.trim()}
                id="btn-query-submit"
              >
                {isQuerying ? (
                  <span className="animate-spin" style={{ display: 'inline-block' }}>⟳</span>
                ) : (
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <line x1="22" y1="2" x2="11" y2="13" />
                    <polygon points="22 2 15 22 11 13 2 9 22 2" />
                  </svg>
                )}
              </button>
            </div>

            {isQuerying && (
              <div className="query-loading animate-fadeIn">
                <div className="query-loading-bar">
                  <div className="query-loading-fill animate-shimmer" />
                </div>
                <span className="query-loading-text">Searching knowledge graph…</span>
              </div>
            )}

            {queryResult && (
              <div className="query-result animate-fadeInUp" id="query-result">
                <div className="query-result-header">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--ng-accent)" strokeWidth="2">
                    <path d="M12 2L2 7l10 5 10-5-10-5z" />
                    <path d="M2 17l10 5 10-5" />
                    <path d="M2 12l10 5 10-5" />
                  </svg>
                  <span>NeuroGraph Response</span>
                </div>
                <div className="query-result-body">
                  {queryResult.split('\n').map((line, i) => (
                    <p key={i}>{line}</p>
                  ))}
                </div>
              </div>
            )}
          </div>

          {/* Quick queries */}
          <div className="query-suggestions">
            <h4 className="sidebar-section-subtitle">Quick Queries</h4>
            {[
              'What technologies does NeuroGraph use?',
              'Who works on AI alignment?',
              'How is NeuroGraph related to GraphRAG?',
              'What organizations are in the graph?',
            ].map((q) => (
              <button
                key={q}
                className="query-suggestion"
                onClick={() => setQueryText(q)}
              >
                <span className="query-suggestion-icon">→</span>
                {q}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// ── Sub-components ──

function EntityDetail({
  entity,
  relationships,
  getEntityName,
}: {
  entity: NonNullable<ReturnType<typeof Array.prototype.find>>;
  relationships: any[];
  getEntityName: (id: string) => string;
}) {
  const e = entity as any;
  const color = getEntityColor(e.entity_type);

  return (
    <div className="entity-detail">
      {/* Header */}
      <div className="entity-header">
        <div
          className="entity-avatar"
          style={{ background: color.fill, color: color.text }}
        >
          {e.name?.charAt(0) || '?'}
        </div>
        <div className="entity-header-info">
          <h3 className="entity-name">{e.name}</h3>
          <span
            className="ng-badge"
            style={{ background: color.badge, color: color.badgeText }}
          >
            {e.entity_type}
          </span>
        </div>
      </div>

      {/* Summary */}
      <div className="sidebar-section">
        <h4 className="sidebar-section-subtitle">Summary</h4>
        <p className="entity-summary">{e.summary}</p>
      </div>

      {/* Metrics */}
      <div className="sidebar-section">
        <h4 className="sidebar-section-subtitle">Metrics</h4>
        <div className="entity-metrics">
          <div className="metric">
            <span className="metric-value">{((e.importance_score || 0) * 100).toFixed(0)}%</span>
            <span className="metric-label">Importance</span>
          </div>
          <div className="metric">
            <span className="metric-value">{e.access_count || 0}</span>
            <span className="metric-label">Accesses</span>
          </div>
          <div className="metric">
            <span className="metric-value">{relationships.length}</span>
            <span className="metric-label">Relations</span>
          </div>
        </div>
      </div>

      {/* Labels */}
      {e.labels?.length > 0 && (
        <div className="sidebar-section">
          <h4 className="sidebar-section-subtitle">Labels</h4>
          <div className="entity-labels">
            {e.labels.map((label: string) => (
              <span key={label} className="ng-badge ng-badge-accent">
                {label}
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Relationships */}
      {relationships.length > 0 && (
        <div className="sidebar-section">
          <h4 className="sidebar-section-subtitle">Relationships</h4>
          <div className="entity-relationships">
            {relationships.map((rel: any) => {
              const isSource = rel.source_entity_id === e.id;
              const otherName = getEntityName(
                isSource ? rel.target_entity_id : rel.source_entity_id
              );
              return (
                <div key={rel.id} className="relationship-item">
                  <span className="rel-direction">{isSource ? '→' : '←'}</span>
                  <span className="rel-type ng-badge ng-badge-cyan">
                    {rel.relationship_type.replace(/_/g, ' ')}
                  </span>
                  <span className="rel-target truncate">{otherName}</span>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Temporal info */}
      <div className="sidebar-section">
        <h4 className="sidebar-section-subtitle">Timeline</h4>
        <div className="entity-timeline">
          <div className="timeline-item">
            <span className="timeline-label">Created</span>
            <span className="timeline-value mono">
              {new Date(e.created_at).toLocaleDateString()}
            </span>
          </div>
          <div className="timeline-item">
            <span className="timeline-label">Updated</span>
            <span className="timeline-value mono">
              {new Date(e.updated_at).toLocaleDateString()}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}

function EdgeDetail({
  edge,
  getEntityName,
}: {
  edge: any;
  getEntityName: (id: string) => string;
}) {
  return (
    <div className="entity-detail">
      <div className="entity-header">
        <div className="entity-avatar" style={{ background: 'hsl(258, 50%, 30%)', color: '#fff' }}>
          ↔
        </div>
        <div className="entity-header-info">
          <h3 className="entity-name">{edge.relationship_type.replace(/_/g, ' ')}</h3>
          <span className="ng-badge ng-badge-cyan">Relationship</span>
        </div>
      </div>

      <div className="sidebar-section">
        <h4 className="sidebar-section-subtitle">Fact</h4>
        <p className="entity-summary">{edge.fact}</p>
      </div>

      <div className="sidebar-section">
        <h4 className="sidebar-section-subtitle">Connection</h4>
        <div className="edge-connection">
          <span className="edge-endpoint">{getEntityName(edge.source_entity_id)}</span>
          <span className="edge-arrow">→</span>
          <span className="edge-endpoint">{getEntityName(edge.target_entity_id)}</span>
        </div>
      </div>

      <div className="sidebar-section">
        <h4 className="sidebar-section-subtitle">Metrics</h4>
        <div className="entity-metrics">
          <div className="metric">
            <span className="metric-value">{edge.weight.toFixed(2)}</span>
            <span className="metric-label">Weight</span>
          </div>
          <div className="metric">
            <span className="metric-value">{(edge.confidence * 100).toFixed(0)}%</span>
            <span className="metric-label">Confidence</span>
          </div>
        </div>
      </div>

      {edge.valid_from && (
        <div className="sidebar-section">
          <h4 className="sidebar-section-subtitle">Validity</h4>
          <div className="entity-timeline">
            <div className="timeline-item">
              <span className="timeline-label">Valid from</span>
              <span className="timeline-value mono">
                {new Date(edge.valid_from).toLocaleDateString()}
              </span>
            </div>
            {edge.valid_until && (
              <div className="timeline-item">
                <span className="timeline-label">Valid until</span>
                <span className="timeline-value mono">
                  {new Date(edge.valid_until).toLocaleDateString()}
                </span>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
