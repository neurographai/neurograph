// ============================================================
// NeuroGraph — TypeScript Type System
// Bridges Rust backend data models ↔ G6 visualization types
// ============================================================

// ── NeuroGraph Domain Types (mirror Rust structs) ──

export interface NeuroEntity {
  id: string;
  name: string;
  entity_type: string;
  summary: string;
  group_id: string;
  labels: string[];
  attributes: Record<string, unknown>;
  importance_score: number;
  access_count: number;
  community_ids: Record<number, string>;
  created_at: string;
  updated_at: string;
  metadata: Record<string, unknown>;
}

export interface NeuroRelationship {
  id: string;
  source_entity_id: string;
  target_entity_id: string;
  relationship_type: string;
  name: string;
  fact: string;
  weight: number;
  confidence: number;
  group_id: string;
  episode_ids: string[];
  valid_from: string;
  valid_until: string | null;
  created_at: string;
  expired_at: string | null;
  attributes: Record<string, unknown>;
}

export interface NeuroCommunity {
  id: string;
  name: string;
  level: number;
  parent_id: string | null;
  children_ids: string[];
  member_entity_ids: string[];
  summary: string;
  group_id: string;
  is_dirty: boolean;
  created_at: string;
  updated_at: string;
  metadata: Record<string, unknown>;
}

export interface NeuroGraphData {
  entities: NeuroEntity[];
  relationships: NeuroRelationship[];
  communities: NeuroCommunity[];
}

// ── G6 Data Types (what G6's Graph receives) ──

export interface G6NodeDatum {
  id: string;
  type?: string;
  combo?: string;
  data?: Record<string, unknown>;
  style?: Record<string, unknown>;
  states?: string[];
}

export interface G6EdgeDatum {
  id: string;
  source: string;
  target: string;
  type?: string;
  data?: Record<string, unknown>;
  style?: Record<string, unknown>;
  states?: string[];
}

export interface G6ComboDatum {
  id: string;
  type?: string;
  combo?: string;
  data?: Record<string, unknown>;
  style?: Record<string, unknown>;
  states?: string[];
}

export interface G6GraphData {
  nodes: G6NodeDatum[];
  edges: G6EdgeDatum[];
  combos: G6ComboDatum[];
}

// ── UI State Types ──

export interface SelectedElement {
  type: 'node' | 'edge' | 'combo';
  id: string;
  data: Record<string, unknown>;
}

export interface SearchResult {
  id: string;
  name: string;
  type: string;
  entityType?: string;
  score: number;
}

export interface GraphStats {
  nodeCount: number;
  edgeCount: number;
  comboCount: number;
  entityTypes: Record<string, number>;
  relationshipTypes: Record<string, number>;
  avgDegree: number;
  density: number;
}

export type LayoutType = 'force' | 'circular' | 'radial' | 'dagre' | 'grid' | 'concentric';

export interface LayoutConfig {
  type: LayoutType;
  label: string;
  icon: string;
}

export const LAYOUT_CONFIGS: LayoutConfig[] = [
  { type: 'force', label: 'Force', icon: '⚡' },
  { type: 'circular', label: 'Circular', icon: '⭕' },
  { type: 'radial', label: 'Radial', icon: '🎯' },
  { type: 'dagre', label: 'Dagre', icon: '📊' },
  { type: 'grid', label: 'Grid', icon: '📐' },
  { type: 'concentric', label: 'Concentric', icon: '🔘' },
];
