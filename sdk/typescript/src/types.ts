// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * TypeScript types mirroring the Rust NeuroGraph API.
 */

/** Configuration for connecting to a NeuroGraph server. */
export interface NeuroGraphConfig {
  /** Base URL of the NeuroGraph API server. */
  url: string;

  /** Optional API key for authentication. */
  apiKey?: string;

  /** Request timeout in milliseconds. Default: 30000. */
  timeoutMs?: number;

  /** Optional group ID for multi-tenant graphs. */
  groupId?: string;
}

/** A unique entity identifier (UUID v4 string). */
export type EntityId = string;

/** An entity in the knowledge graph. */
export interface Entity {
  id: EntityId;
  name: string;
  entityType: string;
  summary: string;
  groupId: string;
  validFrom: string;
  validUntil: string | null;
  createdAt: string;
  properties: Record<string, unknown>;
}

/** A relationship (edge) between two entities. */
export interface Relationship {
  id: string;
  sourceEntityId: EntityId;
  targetEntityId: EntityId;
  relationshipType: string;
  fact: string;
  validFrom: string;
  validUntil: string | null;
  weight: number;
  episodeId: string | null;
  properties: Record<string, unknown>;
}

/** An episode (provenance record) for ingested data. */
export interface Episode {
  id: string;
  source: string;
  sourceType: 'text' | 'json' | 'message';
  content: string;
  groupId: string;
  createdAt: string;
}

/** A community cluster of related entities. */
export interface Community {
  id: string;
  name: string;
  summary: string;
  level: number;
  memberIds: EntityId[];
  parentId: string | null;
}

/** Result from a query operation. */
export interface QueryResult {
  answer: string;
  entities: Entity[];
  relationships: Relationship[];
  communities: Community[];
  confidence: number;
  costUsd: number;
  latencyMs: number;
}

/** A point-in-time view of the knowledge graph. */
export interface TemporalSnapshot {
  timestamp: string;
  entityCount: number;
  relationshipCount: number;
  entities: Entity[];
  relationships: Relationship[];
}

/** Diff between two temporal points. */
export interface TemporalDiff {
  from: string;
  to: string;
  entitiesAdded: Entity[];
  entitiesRemoved: Entity[];
  relationshipsAdded: Relationship[];
  relationshipsInvalidated: Relationship[];
}

/** Result from entity search. */
export interface SearchResult {
  entity: Entity;
  score: number;
}

/** Graph statistics. */
export interface GraphStats {
  entityCount: number;
  relationshipCount: number;
  episodeCount: number;
  communityCount: number;
}

/** Diff between two branches. */
export interface BranchDiff {
  sourceBranch: string;
  targetBranch: string;
  entitiesOnlyInSource: Entity[];
  entitiesOnlyInTarget: Entity[];
  conflictingEntities: Entity[];
}

/** Result from community detection. */
export interface CommunityDetectionResult {
  communities: Community[];
  modularity: number;
  levels: number;
  totalNodes: number;
  durationMs: number;
}
