// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @neurograph/sdk — TypeScript client for NeuroGraph temporal knowledge graph engine.
 *
 * @example
 * ```typescript
 * import { NeuroGraph } from '@neurograph/sdk';
 *
 * const ng = new NeuroGraph({ url: 'http://localhost:8000' });
 *
 * await ng.add('Alice joined Anthropic in March 2026');
 * const result = await ng.query('Where does Alice work?');
 * console.log(result.answer);
 * ```
 *
 * @packageDocumentation
 */

export { NeuroGraph } from './client.js';
export type {
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
export { NeuroGraphError, ConnectionError, QueryError, TimeoutError } from './errors.js';
