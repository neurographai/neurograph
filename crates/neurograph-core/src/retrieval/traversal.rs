// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Graph traversal search — finds related entities by walking the graph.

use crate::drivers::traits::GraphDriver;
use crate::graph::EntityId;

use super::semantic::ScoredEntity;

/// Graph walk search: BFS/DFS from seed nodes with scoring.
pub struct TraversalSearcher;

impl TraversalSearcher {
    /// Search by traversing the graph from seed entities.
    ///
    /// 1. Start from seed entity IDs
    /// 2. BFS up to max_depth hops
    /// 3. Score entities by distance (closer = higher score)
    /// 4. Return top-k results
    pub async fn search(
        seed_ids: &[EntityId],
        max_depth: usize,
        k: usize,
        group_id: Option<&str>,
        driver: &dyn GraphDriver,
    ) -> Result<Vec<ScoredEntity>, TraversalSearchError> {
        let mut all_results = Vec::new();

        for seed_id in seed_ids {
            let subgraph = driver
                .traverse(seed_id, max_depth, group_id)
                .await
                .map_err(|e| TraversalSearchError::DriverError(e.to_string()))?;

            // Score entities by graph distance from seed
            // Entities directly connected get highest scores
            for (i, entity) in subgraph.entities.iter().enumerate() {
                // Skip the seed entity itself
                if entity.id == *seed_id {
                    continue;
                }

                // Score inversely proportional to discovery order (BFS = distance)
                let score = 1.0 / (1.0 + i as f64 * 0.3);

                all_results.push(ScoredEntity {
                    entity: entity.clone(),
                    score,
                    source: "traversal".to_string(),
                });
            }
        }

        // Sort by score descending and take top k
        all_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all_results.truncate(k);

        Ok(all_results)
    }
}

/// Errors from traversal search.
#[derive(Debug, thiserror::Error)]
pub enum TraversalSearchError {
    #[error("Driver error: {0}")]
    DriverError(String),

    #[error("No seed entities provided")]
    NoSeeds,
}
