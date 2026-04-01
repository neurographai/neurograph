// ════════════════════════════════════════════════════════════
// NeuroGraph Dashboard — Settings Types
// ════════════════════════════════════════════════════════════

export interface ProviderConfig {
  provider: string;
  display_name: string;
  configured: boolean;
  healthy: boolean;
  latency_ms?: number;
  model: string;
  error?: string;
}

export type ProviderState = 'empty' | 'testing' | 'connected' | 'error';

export interface ModelInfo {
  id: string;
  display_name: string;
  provider: string;
  context_window: number;
  max_output_tokens: number;
  input_cost_per_1m: number;
  output_cost_per_1m: number;
  speed_tier: 'Instant' | 'Fast' | 'Standard' | 'Slow';
  recommended_for: string[];
}

export interface UsageReport {
  total_input_tokens: number;
  total_output_tokens: number;
  total_calls: number;
  total_cost_usd: number;
  by_prompt_type: PromptTypeUsage[];
}

export interface PromptTypeUsage {
  prompt_type: string;
  input_tokens: number;
  output_tokens: number;
  call_count: number;
  cost_usd: number;
}

export interface RouterConfig {
  strategy: RoutingStrategy;
  preferred_provider?: string;
  fallback_chain: string[];
  budget_limit_usd?: number;
  budget_alert_threshold?: number;
  task_overrides: Record<string, { provider: string; model: string }>;
}

export type RoutingStrategy =
  | 'fixed'
  | 'cost_optimized'
  | 'latency_optimized'
  | 'task_aware'
  | 'fallback';
