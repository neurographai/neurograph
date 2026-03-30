import { useEffect } from 'react';
import GraphCanvas from './components/GraphCanvas';
import { TimelineSlider } from './components/TimelineSlider';
import { QueryPanel } from './components/QueryPanel';
import { BranchDiffPanel } from './components/BranchDiffPanel';
import { NodeDetailPanel } from './components/NodeDetailPanel';
import { GraphViewSwitcher } from './components/GraphViewSwitcher';
import { ThemeToggle } from './components/ThemeToggle';
import { useGraphStore } from './store/graphStore';
import type { G6GraphData } from './types/graph';
import { Activity, Database, Cpu, Wifi } from 'lucide-react';
import logoSrc from './assets/logo.png';
import './App.css';

// ════════════════════════════════════════════════════════════
// NeuroGraph Dashboard — Full 3-Column Layout
// ════════════════════════════════════════════════════════════

// Sample data for demonstration
const SAMPLE_DATA: G6GraphData = {
  nodes: [
    { id: 'alice', data: { label: 'Alice', type: 'Person', importance: 0.9, tier: 'semantic' } },
    { id: 'anthropic', data: { label: 'Anthropic', type: 'Organization', importance: 0.85, tier: 'semantic' } },
    { id: 'deepmind', data: { label: 'DeepMind', type: 'Organization', importance: 0.7, tier: 'semantic' } },
    { id: 'bob', data: { label: 'Bob', type: 'Person', importance: 0.75, tier: 'semantic' } },
    { id: 'google', data: { label: 'Google', type: 'Organization', importance: 0.8, tier: 'semantic' } },
    { id: 'openai', data: { label: 'OpenAI', type: 'Organization', importance: 0.85, tier: 'semantic' } },
    { id: 'fact-1', data: { label: 'Alice joined Anthropic', type: 'Fact', importance: 0.8, tier: 'episodic' } },
    { id: 'fact-2', data: { label: 'Bob moved to OpenAI', type: 'Event', importance: 0.7, tier: 'episodic' } },
  ],
  edges: [
    { id: 'e1', source: 'alice', target: 'anthropic', data: { label: 'employed_by', relationType: 'entity', weight: 0.95 } },
    { id: 'e2', source: 'alice', target: 'fact-1', data: { label: 'subject_of', relationType: 'semantic', weight: 0.9 } },
    { id: 'e3', source: 'fact-1', target: 'anthropic', data: { label: 'about', relationType: 'entity', weight: 0.85 } },
    { id: 'e4', source: 'bob', target: 'google', data: { label: 'previously_at', relationType: 'entity', weight: 0.6 } },
    { id: 'e5', source: 'bob', target: 'openai', data: { label: 'employed_by', relationType: 'entity', weight: 0.9 } },
    { id: 'e6', source: 'fact-2', target: 'bob', data: { label: 'subject_of', relationType: 'semantic', weight: 0.85 } },
    { id: 'e7', source: 'fact-1', target: 'fact-2', data: { label: 'follows', relationType: 'temporal', weight: 0.5 } },
    { id: 'e8', source: 'bob', target: 'fact-2', data: { label: 'causes', relationType: 'causal', weight: 0.7 } },
  ],
  combos: [],
};

export default function App() {
  const { nodes, edges, selectedNode } = useGraphStore();

  // Load sample data into the Zustand store on mount
  useEffect(() => {
    const store = useGraphStore.getState();
    store.setNodes([
      { id: 'alice', label: 'Alice', type: 'entity', importance: 0.9, validFrom: '2024-01-01T00:00:00Z', tier: 'semantic' },
      { id: 'anthropic', label: 'Anthropic', type: 'entity', importance: 0.85, validFrom: '2024-01-01T00:00:00Z', tier: 'semantic' },
      { id: 'deepmind', label: 'DeepMind', type: 'entity', importance: 0.7, validFrom: '2024-01-01T00:00:00Z', tier: 'semantic' },
      { id: 'bob', label: 'Bob', type: 'entity', importance: 0.75, validFrom: '2024-01-01T00:00:00Z', tier: 'semantic' },
      { id: 'google', label: 'Google', type: 'entity', importance: 0.8, validFrom: '2024-01-01T00:00:00Z', tier: 'semantic' },
      { id: 'openai', label: 'OpenAI', type: 'entity', importance: 0.85, validFrom: '2024-01-01T00:00:00Z', tier: 'semantic' },
      { id: 'fact-1', label: 'Alice joined Anthropic', type: 'fact', importance: 0.8, validFrom: '2026-03-01T00:00:00Z', tier: 'episodic' },
      { id: 'fact-2', label: 'Bob moved to OpenAI', type: 'event', importance: 0.7, validFrom: '2026-01-15T00:00:00Z', tier: 'episodic' },
    ]);
    store.setEdges([
      { id: 'e1', source: 'alice', target: 'anthropic', relationType: 'entity', weight: 0.95, label: 'employed_by' },
      { id: 'e2', source: 'alice', target: 'fact-1', relationType: 'semantic', weight: 0.9 },
      { id: 'e3', source: 'fact-1', target: 'anthropic', relationType: 'entity', weight: 0.85 },
      { id: 'e4', source: 'bob', target: 'google', relationType: 'entity', weight: 0.6 },
      { id: 'e5', source: 'bob', target: 'openai', relationType: 'entity', weight: 0.9, label: 'employed_by' },
      { id: 'e6', source: 'fact-2', target: 'bob', relationType: 'semantic', weight: 0.85 },
      { id: 'e7', source: 'fact-1', target: 'fact-2', relationType: 'temporal', weight: 0.5 },
      { id: 'e8', source: 'bob', target: 'fact-2', relationType: 'causal', weight: 0.7, label: 'causes' },
    ]);
  }, []);

  return (
    <div className="ng-app">
      {/* ═══ Header ═══ */}
      <header className="ng-header">
        <div className="ng-header-left">
          <img src={logoSrc} alt="NeuroGraph" className="ng-logo" />
          <span className="ng-title">NeuroGraph</span>
          <span className="ng-version">v0.2.0</span>
        </div>
        <div className="ng-stats-bar">
          <span><Database size={12} /> {nodes.length} nodes</span>
          <span><Activity size={12} /> {edges.length} edges</span>
          <span><Cpu size={12} /> sled</span>
          <span className="ng-status">
            <span className="ng-status-dot" />
            <Wifi size={12} /> Connected
          </span>
          <ThemeToggle />
        </div>
      </header>

      {/* ═══ Main ═══ */}
      <div className="ng-main">
        {/* Left sidebar */}
        <aside className="ng-sidebar-left">
          <QueryPanel />
          <BranchDiffPanel />

          {/* Legend */}
          <div className="ng-panel ng-legend">
            <p className="ng-section-label">Node Types</p>
            {[
              { label: 'Entity', color: '#6366f1' },
              { label: 'Event', color: '#f59e0b' },
              { label: 'Fact', color: '#10b981' },
              { label: 'Concept', color: '#ec4899' },
            ].map(({ label, color }) => (
              <div key={label} className="ng-legend-item">
                <span className="ng-legend-dot" style={{ backgroundColor: color }} />
                {label}
              </div>
            ))}
            <p className="ng-section-label" style={{ marginTop: 12 }}>Memory Tiers</p>
            {[
              { label: 'L1 Working', color: '#fbbf24' },
              { label: 'L2 Episodic', color: '#60a5fa' },
              { label: 'L3 Semantic', color: '#a78bfa' },
              { label: 'L4 Procedural', color: '#34d399' },
            ].map(({ label, color }) => (
              <div key={label} className="ng-legend-item">
                <span className="ng-legend-ring" style={{ borderColor: color }} />
                {label}
              </div>
            ))}
          </div>
        </aside>

        {/* Center: graph + timeline */}
        <main className="ng-center">
          <div className="ng-view-bar"><GraphViewSwitcher /></div>
          <div className="ng-graph-area">
            <GraphCanvas
              data={SAMPLE_DATA}
              layout="force"
              onNodeClick={(id) => {
                const node = nodes.find((n) => n.id === id);
                if (node) useGraphStore.getState().selectNode(node);
              }}
              onCanvasClick={() => useGraphStore.getState().selectNode(null)}
            />
          </div>
          <div className="ng-timeline-area"><TimelineSlider /></div>
        </main>

        {/* Right sidebar: node detail */}
        {selectedNode && <NodeDetailPanel />}
      </div>
    </div>
  );
}
