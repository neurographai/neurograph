// ============================================================
// NeuroGraph — Legend Component
// Visual legend showing entity types and their colors
// ============================================================

import { useState } from 'react';
import { getAllEntityColors } from '../graph/palette';
import type { NeuroGraphData } from '../types/graph';

interface LegendProps {
  data: NeuroGraphData;
  onFilterType?: (type: string | null) => void;
  activeFilter: string | null;
}

export default function Legend({ data, onFilterType, activeFilter }: LegendProps) {
  const [isCollapsed, setIsCollapsed] = useState(false);

  // Count entities per type
  const typeCounts: Record<string, number> = {};
  data.entities.forEach((e) => {
    typeCounts[e.entity_type] = (typeCounts[e.entity_type] || 0) + 1;
  });

  const allColors = getAllEntityColors();
  const types = Object.keys(typeCounts).sort();

  if (isCollapsed) {
    return (
      <div className="ng-legend glass-card" id="legend">
        <button
          className="legend-toggle icon-btn"
          onClick={() => setIsCollapsed(false)}
          title="Show Legend"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
            <line x1="3" y1="9" x2="21" y2="9" />
            <line x1="9" y1="21" x2="9" y2="9" />
          </svg>
        </button>
      </div>
    );
  }

  return (
    <div className="ng-legend glass-card animate-fadeIn" id="legend">
      <div className="legend-header">
        <span className="legend-title">Entity Types</span>
        <button
          className="legend-toggle icon-btn"
          onClick={() => setIsCollapsed(true)}
          title="Collapse Legend"
        >
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <polyline points="6 9 12 15 18 9" />
          </svg>
        </button>
      </div>

      <div className="legend-items">
        {types.map((type) => {
          const color = allColors[type] || { fill: 'hsl(225, 15%, 40%)', badgeText: '#aaa' };
          const isActive = activeFilter === null || activeFilter === type;

          return (
            <button
              key={type}
              className={`legend-item ${isActive ? '' : 'dimmed'} ${activeFilter === type ? 'filtered' : ''}`}
              onClick={() => onFilterType?.(activeFilter === type ? null : type)}
              title={`${type}: ${typeCounts[type]} entities`}
            >
              <span
                className="legend-dot"
                style={{ background: color.fill }}
              />
              <span className="legend-label">{type}</span>
              <span className="legend-count mono">{typeCounts[type]}</span>
            </button>
          );
        })}
      </div>

      {activeFilter && (
        <button
          className="legend-clear"
          onClick={() => onFilterType?.(null)}
        >
          Clear filter
        </button>
      )}
    </div>
  );
}
