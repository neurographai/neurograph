import { create } from 'zustand';

// ════════════════════════════════════════════════════════════
// NeuroGraph Dashboard — Global State (Zustand)
// ════════════════════════════════════════════════════════════

export interface GraphNode {
  id: string;
  label: string;
  type: 'entity' | 'event' | 'fact' | 'concept';
  importance: number;
  validFrom: string;
  validUntil?: string;
  tier: 'working' | 'episodic' | 'semantic' | 'procedural';
  community?: number;
  metadata?: Record<string, unknown>;
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  relationType: 'semantic' | 'temporal' | 'causal' | 'entity';
  weight: number;
  label?: string;
}

export interface Branch {
  name: string;
  createdAt: string;
  nodeCount: number;
  edgeCount: number;
}

export interface QueryResult {
  answer: string;
  sources: GraphNode[];
  reasoning_path: { node: string; relation: string; confidence: number }[];
  cost: { model: string; tokens: number; usd: number; latency_ms: number };
}

export type GraphViewFilter = 'all' | 'semantic' | 'temporal' | 'causal' | 'entity';

export type Theme = 'dark' | 'light';

interface GraphState {
  // Graph data
  nodes: GraphNode[];
  edges: GraphEdge[];
  selectedNode: GraphNode | null;
  hoveredNode: string | null;

  // Time travel
  currentTime: Date;
  timeRange: { min: Date; max: Date };
  isPlaying: boolean;
  playbackSpeed: number;

  // Branches
  branches: Branch[];
  activeBranch: string;
  diffMode: boolean;

  // Query / Search
  queryInput: string;
  queryResults: QueryResult | null;
  isQuerying: boolean;

  // View settings
  activeGraphView: GraphViewFilter;
  theme: Theme;

  // Reasoning animation
  reasoningPath: string[];
  reasoningStep: number;
  isAnimatingReasoning: boolean;

  // Actions
  setNodes: (nodes: GraphNode[]) => void;
  setEdges: (edges: GraphEdge[]) => void;
  selectNode: (node: GraphNode | null) => void;
  setHoveredNode: (id: string | null) => void;
  setCurrentTime: (time: Date) => void;
  togglePlay: () => void;
  setPlaybackSpeed: (speed: number) => void;
  setActiveBranch: (branch: string) => void;
  toggleDiffMode: () => void;
  setQueryInput: (input: string) => void;
  executeQuery: (query: string) => Promise<void>;
  setActiveGraphView: (view: GraphViewFilter) => void;
  toggleTheme: () => void;
  startReasoningAnimation: (path: string[]) => void;
  stepReasoning: () => void;
}

export const useGraphStore = create<GraphState>((set, get) => ({
  nodes: [],
  edges: [],
  selectedNode: null,
  hoveredNode: null,
  currentTime: new Date(),
  timeRange: { min: new Date('2024-01-01'), max: new Date() },
  isPlaying: false,
  playbackSpeed: 1,
  branches: [{ name: 'main', createdAt: new Date().toISOString(), nodeCount: 0, edgeCount: 0 }],
  activeBranch: 'main',
  diffMode: false,
  queryInput: '',
  queryResults: null,
  isQuerying: false,
  activeGraphView: 'all',
  theme: (() => {
    const saved = typeof window !== 'undefined' ? localStorage.getItem('ng-theme') : null;
    const theme: Theme = saved === 'light' ? 'light' : 'dark';
    if (typeof document !== 'undefined') document.documentElement.dataset.theme = theme;
    return theme;
  })(),
  reasoningPath: [],
  reasoningStep: 0,
  isAnimatingReasoning: false,

  setNodes: (nodes) => set({ nodes }),
  setEdges: (edges) => set({ edges }),
  selectNode: (node) => set({ selectedNode: node }),
  setHoveredNode: (id) => set({ hoveredNode: id }),
  setCurrentTime: (time) => set({ currentTime: time }),
  togglePlay: () => set((s) => ({ isPlaying: !s.isPlaying })),
  setPlaybackSpeed: (speed) => set({ playbackSpeed: speed }),
  setActiveBranch: (branch) => set({ activeBranch: branch }),
  toggleDiffMode: () => set((s) => ({ diffMode: !s.diffMode })),
  setQueryInput: (input) => set({ queryInput: input }),

  executeQuery: async (query: string) => {
    set({ isQuerying: true });
    try {
      const res = await fetch('/api/query', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ query, branch: get().activeBranch }),
      });
      const data = await res.json();
      set({ queryResults: data, isQuerying: false });
      if (data.reasoning_path?.length) {
        get().startReasoningAnimation(data.reasoning_path.map((s: { node: string }) => s.node));
      }
    } catch {
      set({ isQuerying: false });
    }
  },

  setActiveGraphView: (view) => set({ activeGraphView: view }),

  toggleTheme: () => set((s) => {
    const next: Theme = s.theme === 'dark' ? 'light' : 'dark';
    localStorage.setItem('ng-theme', next);
    document.documentElement.dataset.theme = next;
    return { theme: next };
  }),

  startReasoningAnimation: (path) =>
    set({ reasoningPath: path, reasoningStep: 0, isAnimatingReasoning: true }),
  stepReasoning: () =>
    set((s) => {
      const next = s.reasoningStep + 1;
      return next >= s.reasoningPath.length
        ? { reasoningStep: next, isAnimatingReasoning: false }
        : { reasoningStep: next };
    }),
}));
