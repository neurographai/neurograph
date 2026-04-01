import React from 'react';
import { useGraphStore, type GraphViewFilter } from '../store/graphStore';
import { Network, Clock, Zap, Users, Layers, Brain } from 'lucide-react';

// ════════════════════════════════════════════════════════════
// Graph View Switcher — toggle the 5 orthogonal graph views
// ════════════════════════════════════════════════════════════

const VIEWS: { key: GraphViewFilter; label: string; icon: React.FC<{ size?: number }>; color: string }[] = [
  { key: 'all', label: 'All', icon: Layers, color: '#e2e8f0' },
  { key: 'semantic', label: 'Semantic', icon: Network, color: '#6366f1' },
  { key: 'temporal', label: 'Temporal', icon: Clock, color: '#f59e0b' },
  { key: 'causal', label: 'Causal', icon: Zap, color: '#ef4444' },
  { key: 'entity', label: 'Entity', icon: Users, color: '#10b981' },
  { key: 'embeddings', label: 'Embeddings', icon: Brain, color: '#00f5d4' },
];

export const GraphViewSwitcher: React.FC = () => {
  const { activeGraphView, setActiveGraphView, edges } = useGraphStore();

  return (
    <div className="ng-view-switcher">
      {VIEWS.map(({ key, label, icon: Icon, color }) => {
        const count = key === 'all' ? edges.length : edges.filter((e) => e.relationType === key).length;
        const isActive = activeGraphView === key;
        return (
          <button
            key={key}
            onClick={() => setActiveGraphView(key)}
            className={`ng-view-btn ${isActive ? 'active' : ''}`}
          >
            <Icon size={14} />
            {label}
            <span className="ng-view-count" style={isActive ? { color } : {}}>{count}</span>
          </button>
        );
      })}
    </div>
  );
};
