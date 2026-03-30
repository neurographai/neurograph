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

// Sample data for demonstration — varied types, tiers, importance, access counts
const SAMPLE_DATA: G6GraphData = {
  nodes: [
    { id: 'alice', data: { label: 'Alice', type: 'Person', importance: 0.95, tier: 'semantic', accessCount: 14 } },
    { id: 'anthropic', data: { label: 'Anthropic', type: 'Organization', importance: 0.88, tier: 'semantic', accessCount: 9 } },
    { id: 'deepmind', data: { label: 'DeepMind', type: 'Organization', importance: 0.55, tier: 'semantic', accessCount: 3 } },
    { id: 'bob', data: { label: 'Bob', type: 'Person', importance: 0.72, tier: 'semantic', accessCount: 7 } },
    { id: 'google', data: { label: 'Google', type: 'Organization', importance: 0.65, tier: 'semantic', accessCount: 5 } },
    { id: 'openai', data: { label: 'OpenAI', type: 'Organization', importance: 0.82, tier: 'semantic', accessCount: 11 } },
    { id: 'fact-1', data: { label: 'Alice joined Anthropic', type: 'Fact', importance: 0.78, tier: 'episodic', accessCount: 6 } },
    { id: 'fact-2', data: { label: 'Bob moved to OpenAI', type: 'Event', importance: 0.60, tier: 'episodic', accessCount: 4 } },
    { id: 'claude', data: { label: 'Claude AI', type: 'Concept', importance: 0.90, tier: 'semantic', accessCount: 12 } },
    { id: 'career-shift', data: { label: 'Career transition wave', type: 'Event', importance: 0.45, tier: 'working', accessCount: 2 } },
    { id: 'rule-1', data: { label: 'Employment lookup rule', type: 'Concept', importance: 0.35, tier: 'procedural', accessCount: 18 } },
  ],
  edges: [
    { id: 'e1', source: 'alice', target: 'anthropic', data: { label: 'employed_by', relationType: 'entity', weight: 0.95 } },
    { id: 'e2', source: 'alice', target: 'fact-1', data: { label: 'subject_of', relationType: 'semantic', weight: 0.9 } },
    { id: 'e3', source: 'fact-1', target: 'anthropic', data: { label: 'about', relationType: 'entity', weight: 0.85 } },
    { id: 'e4', source: 'bob', target: 'google', data: { label: 'previously_at', relationType: 'temporal', weight: 0.5 } },
    { id: 'e5', source: 'bob', target: 'openai', data: { label: 'employed_by', relationType: 'entity', weight: 0.9 } },
    { id: 'e6', source: 'fact-2', target: 'bob', data: { label: 'subject_of', relationType: 'semantic', weight: 0.85 } },
    { id: 'e7', source: 'fact-1', target: 'fact-2', data: { label: 'follows', relationType: 'temporal', weight: 0.5 } },
    { id: 'e8', source: 'bob', target: 'fact-2', data: { label: 'causes', relationType: 'causal', weight: 0.7 } },
    { id: 'e9', source: 'anthropic', target: 'claude', data: { label: 'develops', relationType: 'entity', weight: 0.92 } },
    { id: 'e10', source: 'career-shift', target: 'fact-1', data: { label: 'includes', relationType: 'causal', weight: 0.55 } },
    { id: 'e11', source: 'career-shift', target: 'fact-2', data: { label: 'includes', relationType: 'causal', weight: 0.50 } },
    { id: 'e12', source: 'rule-1', target: 'alice', data: { label: 'applies_to', relationType: 'semantic', weight: 0.40 } },
    { id: 'e13', source: 'rule-1', target: 'bob', data: { label: 'applies_to', relationType: 'semantic', weight: 0.40 } },
    { id: 'e14', source: 'deepmind', target: 'google', data: { label: 'subsidiary_of', relationType: 'entity', weight: 0.88 } },
  ],
  combos: [],
};

export default function App() {
  const { nodes, edges, selectedNode } = useGraphStore();

  // Load sample data into the Zustand store on mount
  useEffect(() => {
    const store = useGraphStore.getState();
    store.setNodes([
      { id: 'alice', label: 'Alice', type: 'entity', importance: 0.95, validFrom: '2024-01-01T00:00:00Z', tier: 'semantic' },
      { id: 'anthropic', label: 'Anthropic', type: 'entity', importance: 0.88, validFrom: '2024-01-01T00:00:00Z', tier: 'semantic' },
      { id: 'deepmind', label: 'DeepMind', type: 'entity', importance: 0.55, validFrom: '2024-01-01T00:00:00Z', tier: 'semantic' },
      { id: 'bob', label: 'Bob', type: 'entity', importance: 0.72, validFrom: '2024-01-01T00:00:00Z', tier: 'semantic' },
      { id: 'google', label: 'Google', type: 'entity', importance: 0.65, validFrom: '2024-01-01T00:00:00Z', tier: 'semantic' },
      { id: 'openai', label: 'OpenAI', type: 'entity', importance: 0.82, validFrom: '2024-01-01T00:00:00Z', tier: 'semantic' },
      { id: 'fact-1', label: 'Alice joined Anthropic', type: 'fact', importance: 0.78, validFrom: '2026-03-01T00:00:00Z', tier: 'episodic' },
      { id: 'fact-2', label: 'Bob moved to OpenAI', type: 'event', importance: 0.60, validFrom: '2026-01-15T00:00:00Z', tier: 'episodic' },
      { id: 'claude', label: 'Claude AI', type: 'entity', importance: 0.90, validFrom: '2024-06-01T00:00:00Z', tier: 'semantic' },
      { id: 'career-shift', label: 'Career transition wave', type: 'event', importance: 0.45, validFrom: '2026-02-01T00:00:00Z', tier: 'working' },
      { id: 'rule-1', label: 'Employment lookup rule', type: 'entity', importance: 0.35, validFrom: '2024-01-01T00:00:00Z', tier: 'procedural' },
    ]);
    store.setEdges([
      { id: 'e1', source: 'alice', target: 'anthropic', relationType: 'entity', weight: 0.95, label: 'employed_by' },
      { id: 'e2', source: 'alice', target: 'fact-1', relationType: 'semantic', weight: 0.9 },
      { id: 'e3', source: 'fact-1', target: 'anthropic', relationType: 'entity', weight: 0.85 },
      { id: 'e4', source: 'bob', target: 'google', relationType: 'temporal', weight: 0.5, label: 'previously_at' },
      { id: 'e5', source: 'bob', target: 'openai', relationType: 'entity', weight: 0.9, label: 'employed_by' },
      { id: 'e6', source: 'fact-2', target: 'bob', relationType: 'semantic', weight: 0.85 },
      { id: 'e7', source: 'fact-1', target: 'fact-2', relationType: 'temporal', weight: 0.5 },
      { id: 'e8', source: 'bob', target: 'fact-2', relationType: 'causal', weight: 0.7, label: 'causes' },
      { id: 'e9', source: 'anthropic', target: 'claude', relationType: 'entity', weight: 0.92, label: 'develops' },
      { id: 'e10', source: 'career-shift', target: 'fact-1', relationType: 'causal', weight: 0.55 },
      { id: 'e11', source: 'career-shift', target: 'fact-2', relationType: 'causal', weight: 0.50 },
      { id: 'e12', source: 'rule-1', target: 'alice', relationType: 'semantic', weight: 0.40 },
      { id: 'e13', source: 'rule-1', target: 'bob', relationType: 'semantic', weight: 0.40 },
      { id: 'e14', source: 'deepmind', target: 'google', relationType: 'entity', weight: 0.88, label: 'subsidiary_of' },
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
              { label: 'Person', color: '#6366f1' },
              { label: 'Organization', color: '#3b82f6' },
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
              { label: 'L1 Working', color: '#f59e0b' },
              { label: 'L2 Episodic', color: '#3b82f6' },
              { label: 'L3 Semantic', color: '#10b981' },
              { label: 'L4 Procedural', color: '#6b7280' },
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
