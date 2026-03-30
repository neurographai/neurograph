// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

import type {
  NeuroGraphConfig,
  QueryResult,
  Entity,
  Relationship,
  Episode,
  Community,
  TemporalSnapshot,
  TemporalDiff,
  SearchResult,
  GraphStats,
  BranchDiff,
  CommunityDetectionResult,
} from './types.js';
import { NeuroGraphError, ConnectionError, QueryError, TimeoutError } from './errors.js';

/**
 * NeuroGraph TypeScript client.
 *
 * Connects to a running NeuroGraph server via REST API.
 * API mirrors the Rust and Python SDKs.
 *
 * @example
 * ```typescript
 * const ng = new NeuroGraph({ url: 'http://localhost:8000' });
 *
 * // Ingest
 * await ng.add('Alice works at Anthropic');
 *
 * // Query
 * const result = await ng.query('Where does Alice work?');
 * console.log(result.answer); // "Anthropic"
 *
 * // Time travel
 * const past = await ng.at('2025-01-01');
 * console.log(past.entityCount);
 *
 * // History
 * const history = await ng.history('Alice');
 * ```
 */
export class NeuroGraph {
  private readonly baseUrl: string;
  private readonly apiKey?: string;
  private readonly timeoutMs: number;
  private readonly groupId?: string;

  constructor(config: NeuroGraphConfig) {
    // Strip trailing slash
    this.baseUrl = config.url.replace(/\/+$/, '');
    this.apiKey = config.apiKey;
    this.timeoutMs = config.timeoutMs ?? 30_000;
    this.groupId = config.groupId;
  }

  // ─── Internal HTTP ─────────────────────────────────────────

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
  ): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      'User-Agent': '@neurograph/sdk',
    };

    if (this.apiKey) {
      headers['Authorization'] = `Bearer ${this.apiKey}`;
    }

    if (this.groupId) {
      headers['X-Group-Id'] = this.groupId;
    }

    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), this.timeoutMs);

    try {
      const response = await fetch(url, {
        method,
        headers,
        body: body ? JSON.stringify(body) : undefined,
        signal: controller.signal,
      });

      if (!response.ok) {
        const errorBody = await response.text().catch(() => 'Unknown error');
        throw new QueryError(
          `${method} ${path} failed (${response.status}): ${errorBody}`,
          response.status,
        );
      }

      return (await response.json()) as T;
    } catch (error: unknown) {
      if (error instanceof NeuroGraphError) throw error;

      if (
        typeof DOMException !== 'undefined' &&
        error instanceof DOMException &&
        error.name === 'AbortError'
      ) {
        throw new TimeoutError(this.timeoutMs);
      }

      const message = error instanceof Error ? error.message : String(error);
      throw new ConnectionError(url, message);
    } finally {
      clearTimeout(timeout);
    }
  }

  // ─── Health ────────────────────────────────────────────────

  /** Check if the NeuroGraph server is reachable. */
  async health(): Promise<boolean> {
    try {
      await this.request<{ status: string }>('GET', '/health');
      return true;
    } catch {
      return false;
    }
  }

  // ─── Ingestion ─────────────────────────────────────────────

  /**
   * Add text to the knowledge graph.
   *
   * The server will extract entities and relationships,
   * deduplicate, and store with embeddings.
   */
  async add(text: string): Promise<Episode> {
    return this.request<Episode>('POST', '/api/v1/ingest/text', { text });
  }

  /**
   * Add a JSON object to the knowledge graph.
   *
   * Entities and relationships are extracted from the JSON structure.
   */
  async addJson(data: Record<string, unknown>): Promise<Episode> {
    return this.request<Episode>('POST', '/api/v1/ingest/json', { data });
  }

  /**
   * Add text with an explicit timestamp.
   *
   * @param text - The text to ingest.
   * @param date - ISO date string (e.g., "2025-06-15").
   */
  async addAt(text: string, date: string): Promise<Episode> {
    return this.request<Episode>('POST', '/api/v1/ingest/text', {
      text,
      timestamp: date,
    });
  }

  // ─── Query ─────────────────────────────────────────────────

  /**
   * Query the knowledge graph with natural language.
   *
   * Routes to the optimal retrieval strategy based on query type.
   */
  async query(question: string): Promise<QueryResult> {
    return this.request<QueryResult>('POST', '/api/v1/query', { question });
  }

  // ─── Search ────────────────────────────────────────────────

  /**
   * Search entities by text.
   *
   * Uses hybrid search: vector similarity + BM25.
   */
  async search(query: string, limit = 10): Promise<SearchResult[]> {
    return this.request<SearchResult[]>(
      'POST',
      `/api/v1/search?limit=${limit}`,
      { query },
    );
  }

  /**
   * Get an entity by ID.
   */
  async getEntity(id: string): Promise<Entity> {
    return this.request<Entity>('GET', `/api/v1/entities/${id}`);
  }

  /**
   * Get relationships for an entity.
   */
  async getRelationships(entityId: string): Promise<Relationship[]> {
    return this.request<Relationship[]>(
      'GET',
      `/api/v1/entities/${entityId}/relationships`,
    );
  }

  // ─── Temporal ──────────────────────────────────────────────

  /**
   * Get a temporal snapshot at a specific date.
   *
   * @param date - ISO date string (e.g., "2025-01-15", "2025", "2025-01-15T10:30:00Z").
   */
  async at(date: string): Promise<TemporalSnapshot> {
    return this.request<TemporalSnapshot>('GET', `/api/v1/temporal/at/${date}`);
  }

  /**
   * Get the history of an entity (all relationships, including invalidated).
   *
   * @param entityName - The name of the entity.
   */
  async history(entityName: string): Promise<Relationship[]> {
    return this.request<Relationship[]>(
      'GET',
      `/api/v1/temporal/history/${encodeURIComponent(entityName)}`,
    );
  }

  /**
   * Show what changed between two dates.
   */
  async whatChanged(from: string, to: string): Promise<TemporalDiff> {
    return this.request<TemporalDiff>(
      'GET',
      `/api/v1/temporal/diff?from=${from}&to=${to}`,
    );
  }

  // ─── Branching ─────────────────────────────────────────────

  /**
   * Create a new branch of the knowledge graph.
   *
   * @param name - Branch name (e.g., "hypothesis", "what-if").
   */
  async branch(name: string): Promise<void> {
    await this.request<void>('POST', '/api/v1/branches', { name });
  }

  /**
   * Diff two branches.
   */
  async diffBranches(source: string, target: string): Promise<BranchDiff> {
    return this.request<BranchDiff>(
      'GET',
      `/api/v1/branches/diff?source=${source}&target=${target}`,
    );
  }

  // ─── Communities ───────────────────────────────────────────

  /**
   * Run community detection on the graph.
   */
  async detectCommunities(): Promise<CommunityDetectionResult> {
    return this.request<CommunityDetectionResult>(
      'POST',
      '/api/v1/communities/detect',
    );
  }

  /**
   * List all communities.
   */
  async listCommunities(): Promise<Community[]> {
    return this.request<Community[]>('GET', '/api/v1/communities');
  }

  // ─── Stats ─────────────────────────────────────────────────

  /**
   * Get graph statistics.
   */
  async stats(): Promise<GraphStats> {
    return this.request<GraphStats>('GET', '/api/v1/stats');
  }
}
