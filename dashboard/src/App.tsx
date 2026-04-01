import { useEffect, useState } from 'react';
import GraphCanvas from './components/GraphCanvas';
import { TimelineSlider } from './components/TimelineSlider';
import { QueryPanel } from './components/QueryPanel';
import { BranchDiffPanel } from './components/BranchDiffPanel';
import { NodeDetailPanel } from './components/NodeDetailPanel';
import { GraphViewSwitcher } from './components/GraphViewSwitcher';
import { EmbeddingPanel } from './components/EmbeddingPanel';
import { ThemeToggle } from './components/ThemeToggle';
import { ChatPanel } from './components/ChatPanel';
import { SettingsModal, SettingsTrigger } from './components/SettingsModal';
import { useGraphStore } from './store/graphStore';
import type { G6GraphData } from './types/graph';
import { Activity, Database, Cpu, Wifi, WifiOff, FileText, Loader2 } from 'lucide-react';
import logoSrc from './assets/logo.png';
import './App.css';

// ════════════════════════════════════════════════════════════
// NeuroGraph Dashboard — Full 3-Column Layout
// Wired to neurograph-core REST API at /api/v1
// ════════════════════════════════════════════════════════════

export default function App() {
  const { nodes, edges, selectedNode, activeGraphView, serverStats, connectionStatus } = useGraphStore();
  const [settingsOpen, setSettingsOpen] = useState(false);

  // Fetch real stats + graph data from the core server on mount + poll stats every 10s
  useEffect(() => {
    const store = useGraphStore.getState();
    store.fetchStats();
    store.fetchGraph();
    const interval = setInterval(() => {
      useGraphStore.getState().fetchStats();
    }, 10_000);
    return () => clearInterval(interval);
  }, []);

  // Build G6-compatible graph data from store
  const graphData: G6GraphData = {
    nodes: nodes.map((n) => ({
      id: n.id,
      data: {
        label: n.label,
        type: n.type,
        importance: n.importance,
        tier: n.tier,
        accessCount: 0,
      },
    })),
    edges: edges.map((e) => ({
      id: e.id,
      source: e.source,
      target: e.target,
      data: {
        label: e.label,
        relationType: e.relationType,
        weight: e.weight,
      },
    })),
    combos: [],
  };

  // Stats display — use live server stats when available, fall back to local counts
  const displayNodes = serverStats?.entities ?? nodes.length;
  const displayEdges = serverStats?.relationships ?? edges.length;
  const displayEngine = 'sled';

  // Connection status indicator
  const StatusIcon = connectionStatus === 'connected' ? Wifi
    : connectionStatus === 'checking' ? Loader2
    : WifiOff;
  const statusLabel = connectionStatus === 'connected' ? 'Connected'
    : connectionStatus === 'checking' ? 'Connecting…'
    : 'Disconnected';

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
          <span><Database size={12} /> {displayNodes} nodes</span>
          <span><Activity size={12} /> {displayEdges} edges</span>
          {serverStats && <span><FileText size={12} /> {serverStats.papers} papers</span>}
          <span><Cpu size={12} /> {displayEngine}</span>
          <span className={`ng-status ${connectionStatus !== 'connected' ? 'ng-status--offline' : ''}`}>
            <span className={`ng-status-dot ${connectionStatus === 'connected' ? '' : 'ng-status-dot--offline'}`} />
            <StatusIcon size={12} className={connectionStatus === 'checking' ? 'ng-spinner' : ''} />
            {statusLabel}
          </span>
          <ThemeToggle />
          <SettingsTrigger onClick={() => setSettingsOpen(true)} />
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
          {activeGraphView === 'embeddings' ? (
            <div className="ng-graph-area" style={{ overflow: 'auto' }}>
              <EmbeddingPanel />
            </div>
          ) : (
            <>
              <div className="ng-graph-area">
                {nodes.length === 0 ? (
                  <div className="ng-empty-state">
                    <Database size={48} strokeWidth={1} />
                    <h3>No graph data yet</h3>
                    <p>Ingest a PDF or add text via the API to populate the knowledge graph.</p>
                    <p className="ng-empty-hint">
                      Switch to the <strong>Embeddings</strong> view to upload and visualize a PDF.
                    </p>
                  </div>
                ) : (
                  <GraphCanvas
                    data={graphData}
                    layout="force"
                    onNodeClick={(id) => {
                      const node = nodes.find((n) => n.id === id);
                      if (node) useGraphStore.getState().selectNode(node);
                    }}
                    onCanvasClick={() => useGraphStore.getState().selectNode(null)}
                  />
                )}
              </div>
              <div className="ng-timeline-area"><TimelineSlider /></div>
            </>
          )}
        </main>

        {/* Right sidebar: node detail */}
        {selectedNode && <NodeDetailPanel />}
      </div>

      {/* Chat FAB + panel */}
      <ChatPanel />

      {/* Settings modal */}
      {settingsOpen && <SettingsModal onClose={() => setSettingsOpen(false)} />}
    </div>
  );
}
