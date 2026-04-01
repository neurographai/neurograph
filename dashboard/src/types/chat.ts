// ════════════════════════════════════════════════════════════
// NeuroGraph Dashboard — Chat Agent Types
// Mirrors Rust structs from chat/response.rs
// ════════════════════════════════════════════════════════════

export interface ChatAgentRequest {
  message: string;
  session_id?: string;
}

export interface ChatAgentResponse {
  answer: string;
  confidence: number;
  evidence: EvidenceChunk[];
  graph_actions: GraphAction[];
  follow_ups: FollowUpQuestion[];
  meta: ResponseMeta;
}

export interface EvidenceChunk {
  text: string;
  source: EvidenceSource;
  relevance_score: number;
}

export type EvidenceSource =
  | { type: 'paper'; title: string; section: string; page: number }
  | { type: 'entity'; entity_id: string; entity_name: string }
  | { type: 'community'; community_id: number; topic: string }
  | { type: 'temporal'; timestamp: string; description: string };

export interface GraphAction {
  action_type: GraphActionType;
  description: string;
}

export type GraphActionType =
  | { highlight_nodes: { node_ids: string[] } }
  | { expand_subgraph: { center_node_id: string; depth: number } }
  | { switch_view: { view: string } }
  | { filter_edges: { edge_types: string[] } }
  | { jump_timeline: { timestamp: string } }
  | { open_node_panel: { node_id: string } }
  | { focus_nodes: { node_ids: string[] } }
  | 'reset_view';

export interface FollowUpQuestion {
  question: string;
  intent_hint: ChatIntent;
  rationale: string;
}

export interface ResponseMeta {
  intent: ClassifiedIntent;
  tools_used: string[];
  model_used: string;
  input_tokens: number;
  output_tokens: number;
  cost_usd: number;
  latency_ms: number;
  session_id: string;
}

export interface ClassifiedIntent {
  intent: ChatIntent;
  confidence: number;
  method: 'regex' | 'llm';
  extracted_entities: string[];
}

export type ChatIntent =
  | 'explain'
  | 'explore'
  | 'temporal_compare'
  | 'time_travel'
  | 'find_contradictions'
  | 'summarize'
  | 'search'
  | 'trace_relationship'
  | 'discover_themes'
  | 'filter_graph'
  | 'general';

export interface IntentPreview {
  intent: ChatIntent;
  confidence: number;
  label: string;
  entities: string[];
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: string;
  // Assistant-only fields
  response?: ChatAgentResponse;
  isStreaming?: boolean;
}

export interface ChatSession {
  id: string;
  message_count: number;
  created_at: string;
  updated_at: string;
}

export const INTENT_LABELS: Record<ChatIntent, string> = {
  explain: 'Explaining',
  explore: 'Exploring',
  temporal_compare: 'Comparing Timeline',
  time_travel: 'Time Travelling',
  find_contradictions: 'Finding Contradictions',
  summarize: 'Summarizing',
  search: 'Searching',
  trace_relationship: 'Tracing Relationships',
  discover_themes: 'Discovering Themes',
  filter_graph: 'Filtering Graph',
  general: 'Thinking',
};

export const INTENT_COLORS: Record<ChatIntent, string> = {
  explain: '#6366f1',
  explore: '#3b82f6',
  temporal_compare: '#f59e0b',
  time_travel: '#8b5cf6',
  find_contradictions: '#ef4444',
  summarize: '#10b981',
  search: '#06b6d4',
  trace_relationship: '#ec4899',
  discover_themes: '#14b8a6',
  filter_graph: '#f97316',
  general: '#6b7280',
};
