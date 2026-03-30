import React from 'react';
import { useGraphStore } from '../store/graphStore';
import { X, Clock, Layers, Activity, Link2 } from 'lucide-react';
import { format } from 'date-fns';
import { motion, AnimatePresence } from 'framer-motion';

// ════════════════════════════════════════════════════════════
// Node Detail Panel — full inspector when a node is selected
// ════════════════════════════════════════════════════════════

const TYPE_COLORS: Record<string, string> = {
  entity: '#6366f1', event: '#f59e0b', fact: '#10b981', concept: '#ec4899',
};

export const NodeDetailPanel: React.FC = () => {
  const { selectedNode, selectNode, edges, nodes } = useGraphStore();
  if (!selectedNode) return null;

  const connected = edges.filter(
    (e) => e.source === selectedNode.id || e.target === selectedNode.id
  );
  const byType = connected.reduce((acc, e) => {
    (acc[e.relationType] = acc[e.relationType] || []).push(e);
    return acc;
  }, {} as Record<string, typeof connected>);

  return (
    <AnimatePresence>
      <motion.aside
        initial={{ x: 320, opacity: 0 }}
        animate={{ x: 0, opacity: 1 }}
        exit={{ x: 320, opacity: 0 }}
        className="ng-detail-panel"
      >
        <div className="ng-detail-header">
          <div>
            <h3 className="ng-detail-name">{selectedNode.label}</h3>
            <span className="ng-type-badge" style={{
              backgroundColor: (TYPE_COLORS[selectedNode.type] || '#6b7280') + '20',
              color: TYPE_COLORS[selectedNode.type] || '#6b7280',
            }}>
              {selectedNode.type}
            </span>
          </div>
          <button onClick={() => selectNode(null)} className="ng-btn-icon"><X size={16} /></button>
        </div>

        <div className="ng-detail-body">
          {/* Temporal */}
          <section>
            <h4 className="ng-section-label"><Clock size={12} /> Temporal</h4>
            <div className="ng-detail-grid">
              <div>
                <span className="ng-meta-label">Valid from</span>
                <p className="ng-meta-value">{format(new Date(selectedNode.validFrom), 'MMM d, yyyy')}</p>
              </div>
              <div>
                <span className="ng-meta-label">Valid until</span>
                <p className="ng-meta-value">
                  {selectedNode.validUntil
                    ? format(new Date(selectedNode.validUntil), 'MMM d, yyyy')
                    : '∞ (current)'}
                </p>
              </div>
            </div>
          </section>

          {/* Importance */}
          <section>
            <h4 className="ng-section-label"><Activity size={12} /> Importance</h4>
            <div className="ng-importance-bar-wrap">
              <div className="ng-importance-bar">
                <div className="ng-importance-fill" style={{ width: `${selectedNode.importance * 100}%` }} />
              </div>
              <span className="ng-importance-val">{(selectedNode.importance * 100).toFixed(0)}%</span>
            </div>
            <div className="ng-tier-info">
              <Layers size={12} /> <span>{selectedNode.tier} memory</span>
            </div>
          </section>

          {/* Connections */}
          <section>
            <h4 className="ng-section-label"><Link2 size={12} /> Connections ({connected.length})</h4>
            {Object.entries(byType).map(([type, typeEdges]) => (
              <div key={type} className="ng-conn-group">
                <span className="ng-conn-type">{type} ({typeEdges.length})</span>
                {typeEdges.slice(0, 5).map((edge) => {
                  const otherId = edge.source === selectedNode.id ? edge.target : edge.source;
                  const other = nodes.find((n) => n.id === otherId);
                  return (
                    <button
                      key={edge.id}
                      onClick={() => other && selectNode(other)}
                      className="ng-conn-link"
                    >
                      <span>{other?.label ?? otherId}</span>
                      <span className="ng-conn-weight">{(edge.weight * 100).toFixed(0)}%</span>
                    </button>
                  );
                })}
              </div>
            ))}
          </section>
        </div>
      </motion.aside>
    </AnimatePresence>
  );
};
