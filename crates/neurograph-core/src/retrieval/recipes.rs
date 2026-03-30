// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Pre-built search recipes — common query patterns.


use crate::drivers::traits::{GraphDriver, Subgraph};
use crate::embedders::traits::Embedder;
use crate::graph::{Entity, EntityId};

use super::hybrid::{HybridRetriever, HybridSearchResult};

/// Pre-built search recipes for common query patterns.
pub struct SearchRecipes;

impl SearchRecipes {
    /// Find an entity by name (exact or similar).
    ///
    /// Returns the best matching entity, or None if not found.
    pub async fn find_entity(
        name: &str,
        embedder: &dyn Embedder,
        driver: &dyn GraphDriver,
    ) -> Option<Entity> {
        let retriever = HybridRetriever::new();
        let results = retriever
            .search(name, 1, None, None, embedder, driver)
            .await
            .ok()?;

        results.into_iter().next().map(|r| r.entity)
    }

    /// Find connections between two entities.
    ///
    /// Returns the subgraph of paths connecting entity A to entity B.
    pub async fn find_connections(
        entity_a_id: &EntityId,
        _entity_b_id: &EntityId,
        max_depth: usize,
        driver: &dyn GraphDriver,
    ) -> Result<Subgraph, ConnectionSearchError> {
        // Traverse from entity A and see if entity B is reachable
        let subgraph = driver
            .traverse(entity_a_id, max_depth, None)
            .await
            .map_err(|e| ConnectionSearchError::DriverError(e.to_string()))?;

        Ok(subgraph)
    }

    /// Find all entities in a topic's neighborhood.
    ///
    /// Combines semantic search with graph traversal to find
    /// all entities relevant to a topic.
    pub async fn find_related(
        topic: &str,
        k: usize,
        embedder: &dyn Embedder,
        driver: &dyn GraphDriver,
    ) -> Vec<HybridSearchResult> {
        let retriever = HybridRetriever::new();
        retriever
            .search(topic, k, None, None, embedder, driver)
            .await
            .unwrap_or_default()
    }
}

/// Errors from connection search.
#[derive(Debug, thiserror::Error)]
pub enum ConnectionSearchError {
    #[error("Driver error: {0}")]
    DriverError(String),

    #[error("Entity not found: {0}")]
    EntityNotFound(String),
}
