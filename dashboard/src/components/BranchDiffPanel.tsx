import React, { useMemo } from 'react';
import { useGraphStore } from '../store/graphStore';
import { GitBranch, Plus, Minus, ArrowRightLeft } from 'lucide-react';

// ════════════════════════════════════════════════════════════
// Branch Diff Panel — Git-like branch management with visual diff
// ════════════════════════════════════════════════════════════

export const BranchDiffPanel: React.FC = () => {
  const {
    branches, activeBranch, diffMode,
    setActiveBranch, toggleDiffMode,
  } = useGraphStore();

  const diff = useMemo(() => {
    if (!diffMode) return null;
    return {
      added: [
        { id: 'n-1', label: 'Alice → DeepMind', type: 'fact' },
        { id: 'n-2', label: 'DeepMind', type: 'entity' },
      ],
      removed: [
        { id: 'n-3', label: 'Alice → Anthropic', type: 'fact' },
      ],
      modified: [
        { id: 'n-4', label: 'Alice', type: 'entity', change: 'employer: Anthropic → DeepMind' },
      ],
    };
  }, [diffMode]);

  return (
    <div className="ng-panel">
      <div className="ng-panel-header">
        <div className="ng-panel-title"><GitBranch size={14} /> Branches</div>
        <button onClick={toggleDiffMode} className={`ng-diff-toggle ${diffMode ? 'active' : ''}`}>
          <ArrowRightLeft size={12} /> Diff
        </button>
      </div>

      <div className="ng-branch-list">
        {branches.map((branch) => (
          <button
            key={branch.name}
            onClick={() => setActiveBranch(branch.name)}
            className={`ng-branch-btn ${activeBranch === branch.name ? 'active' : ''}`}
          >
            <span>{branch.name}</span>
            <span className="ng-branch-count">{branch.nodeCount} nodes</span>
          </button>
        ))}
      </div>

      {diffMode && diff && (
        <div className="ng-diff-view">
          <p className="ng-section-label">main vs what-if</p>
          {diff.added.map((item) => (
            <div key={item.id} className="ng-diff-row added">
              <Plus size={12} /> <span>{item.label}</span>
              <span className="ng-diff-type">{item.type}</span>
            </div>
          ))}
          {diff.removed.map((item) => (
            <div key={item.id} className="ng-diff-row removed">
              <Minus size={12} /> <span className="ng-strikethrough">{item.label}</span>
              <span className="ng-diff-type">{item.type}</span>
            </div>
          ))}
          {diff.modified.map((item) => (
            <div key={item.id} className="ng-diff-row modified">
              <ArrowRightLeft size={12} />
              <div><span>{item.label}</span> <span className="ng-diff-change">{item.change}</span></div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
