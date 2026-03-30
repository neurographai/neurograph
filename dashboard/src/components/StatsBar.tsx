// ============================================================
// NeuroGraph — StatsBar Component
// Bottom status bar with graph stats and live metrics
// ============================================================

import { useMemo } from 'react';
import type { NeuroGraphData, GraphStats } from '../types/graph';

interface StatsBarProps {
  data: NeuroGraphData;
}

export default function StatsBar({ data }: StatsBarProps) {
  const stats = useMemo<GraphStats>(() => {
    const entityTypes: Record<string, number> = {};
    const relationshipTypes: Record<string, number> = {};

    data.entities.forEach((e) => {
      entityTypes[e.entity_type] = (entityTypes[e.entity_type] || 0) + 1;
    });

    data.relationships.forEach((r) => {
      relationshipTypes[r.relationship_type] =
        (relationshipTypes[r.relationship_type] || 0) + 1;
    });

    const nodeCount = data.entities.length;
    const edgeCount = data.relationships.length;
    const comboCount = data.communities.length;
    const avgDegree = nodeCount > 0 ? (2 * edgeCount) / nodeCount : 0;
    const maxEdges = nodeCount * (nodeCount - 1);
    const density = maxEdges > 0 ? edgeCount / maxEdges : 0;

    return {
      nodeCount,
      edgeCount,
      comboCount,
      entityTypes,
      relationshipTypes,
      avgDegree,
      density,
    };
  }, [data]);

  return (
    <div className="ng-statsbar" id="stats-bar">
      <div className="stat-group">
        <span className="stat-icon">◉</span>
        <span className="stat-label">Nodes</span>
        <span className="stat-value mono">{stats.nodeCount}</span>
      </div>

      <div className="stat-group">
        <span className="stat-icon">⟶</span>
        <span className="stat-label">Edges</span>
        <span className="stat-value mono">{stats.edgeCount}</span>
      </div>

      <div className="stat-group">
        <span className="stat-icon">◎</span>
        <span className="stat-label">Communities</span>
        <span className="stat-value mono">{stats.comboCount}</span>
      </div>

      <div className="stat-divider" />

      <div className="stat-group">
        <span className="stat-label">Types</span>
        <span className="stat-value mono">{Object.keys(stats.entityTypes).length}</span>
      </div>

      <div className="stat-group">
        <span className="stat-label">Avg Degree</span>
        <span className="stat-value mono">{stats.avgDegree.toFixed(1)}</span>
      </div>

      <div className="stat-group">
        <span className="stat-label">Density</span>
        <span className="stat-value mono">{(stats.density * 100).toFixed(1)}%</span>
      </div>

      <div className="stat-spacer" />

      <div className="stat-group stat-status">
        <span className="stat-status-dot" />
        <span className="stat-label">NeuroGraph v0.1.0</span>
      </div>
    </div>
  );
}
