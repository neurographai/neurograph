import { create } from 'zustand';

// ════════════════════════════════════════════════════════════
// NeuroGraph Dashboard — Global State (Zustand)
// ════════════════════════════════════════════════════════════

// API base — relative URL works with Vite proxy in dev and same-origin in prod
const API_BASE = '/api/v1';

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

export interface ServerStats {
  entities: number;
  relationships: number;
  papers: number;
  episodes: number;
}

export type GraphViewFilter = 'all' | 'semantic' | 'temporal' | 'causal' | 'entity' | 'embeddings';

export type Theme = 'dark' | 'light';

export type ConnectionStatus = 'connected' | 'disconnected' | 'checking';

interface GraphState {
  // Graph data
  nodes: GraphNode[];
  edges: GraphEdge[];
  selectedNode: GraphNode | null;
  hoveredNode: string | null;

  // Server stats (live from API)
  serverStats: ServerStats | null;
  connectionStatus: ConnectionStatus;

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
  fetchStats: () => Promise<void>;
  fetchGraph: () => Promise<void>;
  checkConnection: () => Promise<void>;
}

export const useGraphStore = create<GraphState>((set, get) => ({
  nodes: [],
  edges: [],
  selectedNode: null,
  hoveredNode: null,
  serverStats: null,
  connectionStatus: 'checking',
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
      const res = await fetch(`${API_BASE}/query`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ query, branch: get().activeBranch }),
      });
      const wrapper = await res.json();
      // The core API wraps responses in { success, data, error }
      const data = wrapper.data ?? wrapper;
      set({
        queryResults: {
          answer: data.answer ?? 'No answer available.',
          sources: data.sources ?? [],
          reasoning_path: data.reasoning_path ?? [],
          cost: data.cost ?? null,
        },
        isQuerying: false,
      });

      // Merge query-returned entities into the visible graph
      if (data.entities?.length) {
        const existing = get().nodes;
        const existingIds = new Set(existing.map((n) => n.id));
        const newNodes: GraphNode[] = data.entities
          .filter((e: Record<string, unknown>) => !existingIds.has(e.id as string))
          .map((e: Record<string, unknown>) => ({
            id: e.id as string,
            label: (e.name as string) ?? (e.id as string),
            type: mapEntityType(e.entity_type as string),
            importance: (e.importance_score as number) ?? 0.5,
            validFrom: (e.created_at as string) ?? new Date().toISOString(),
            tier: 'semantic' as const,
          }));
        if (newNodes.length > 0) set({ nodes: [...existing, ...newNodes] });
      }
      if (data.relationships?.length) {
        const existingEdges = get().edges;
        const existingEdgeIds = new Set(existingEdges.map((e) => e.id));
        const newEdges: GraphEdge[] = data.relationships
          .filter((r: Record<string, unknown>) => !existingEdgeIds.has(r.id as string))
          .map((r: Record<string, unknown>) => ({
            id: r.id as string,
            source: r.source_entity_id as string,
            target: r.target_entity_id as string,
            relationType: mapRelationType(r.relationship_type as string),
            weight: (r.weight as number) ?? 0.5,
            label: r.name as string,
          }));
        if (newEdges.length > 0) set({ edges: [...existingEdges, ...newEdges] });
      }

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

  fetchStats: async () => {
    try {
      const res = await fetch(`${API_BASE}/stats`);
      const wrapper = await res.json();
      const data = wrapper.data ?? wrapper;
      set({
        serverStats: {
          entities: data.entities ?? 0,
          relationships: data.relationships ?? 0,
          papers: data.papers ?? 0,
          episodes: data.episodes ?? 0,
        },
        connectionStatus: 'connected',
      });
    } catch {
      set({ connectionStatus: 'disconnected' });
    }
  },

  fetchGraph: async () => {
    try {
      const res = await fetch(`${API_BASE}/graph?limit=500`);
      const wrapper = await res.json();
      const data = wrapper.data ?? wrapper;

      const entities = data.entities ?? [];
      const relationships = data.relationships ?? [];

      const nodes: GraphNode[] = entities.map((e: Record<string, unknown>) => ({
        id: e.id as string,
        label: (e.name as string) ?? (e.id as string),
        type: mapEntityType(e.entity_type as string),
        importance: (e.importance_score as number) ?? 0.5,
        validFrom: (e.created_at as string) ?? new Date().toISOString(),
        validUntil: undefined,
        tier: 'semantic' as const,
        community: undefined,
        metadata: (e.attributes as Record<string, unknown>) ?? {},
      }));

      const edges: GraphEdge[] = relationships.map((r: Record<string, unknown>) => ({
        id: r.id as string,
        source: r.source_entity_id as string,
        target: r.target_entity_id as string,
        relationType: mapRelationType(r.relationship_type as string),
        weight: (r.weight as number) ?? 0.5,
        label: r.name as string,
      }));

      set({ nodes, edges });
    } catch {
      // Server may not be running; leave graph empty
    }
  },

  checkConnection: async () => {
    set({ connectionStatus: 'checking' });
    try {
      const res = await fetch(`${API_BASE}/health`);
      if (res.ok) {
        set({ connectionStatus: 'connected' });
      } else {
        set({ connectionStatus: 'disconnected' });
      }
    } catch {
      set({ connectionStatus: 'disconnected' });
    }
  },
}));

// ── Helpers: map server entity/relationship types to store types ──

function mapEntityType(serverType: string): GraphNode['type'] {
  const t = (serverType ?? '').toLowerCase();
  if (t.includes('event')) return 'event';
  if (t.includes('fact')) return 'fact';
  if (t.includes('concept')) return 'concept';
  return 'entity';
}

function mapRelationType(serverType: string): GraphEdge['relationType'] {
  const t = (serverType ?? '').toLowerCase();
  if (t.includes('temporal') || t.includes('time')) return 'temporal';
  if (t.includes('caus')) return 'causal';
  if (t.includes('entity') || t.includes('has') || t.includes('is')) return 'entity';
  return 'semantic';
}
