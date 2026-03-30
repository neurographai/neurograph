// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Semantic (vector) search over the knowledge graph.

use crate::drivers::traits::{GraphDriver, VectorSearchResult};
use crate::embedders::traits::Embedder;

/// Semantic search: finds entities by embedding similarity.
pub struct SemanticSearcher;

impl SemanticSearcher {
    /// Search for entities similar to the query text.
    ///
    /// 1. Embed the query text
    /// 2. Search the driver's vector index
    /// 3. Return scored results
    pub async fn search(
        query: &str,
        k: usize,
        group_id: Option<&str>,
        embedder: &dyn Embedder,
        driver: &dyn GraphDriver,
    ) -> Result<Vec<ScoredEntity>, SemanticSearchError> {
        // Generate query embedding
        let embedding = embedder
            .embed_one(query)
            .await
            .map_err(|e| SemanticSearchError::EmbedderError(e.to_string()))?;

        Self::search_by_vector(&embedding, k, group_id, driver).await
    }

    /// Search with a pre-computed embedding vector.
    pub async fn search_by_vector(
        embedding: &[f32],
        k: usize,
        group_id: Option<&str>,
        driver: &dyn GraphDriver,
    ) -> Result<Vec<ScoredEntity>, SemanticSearchError> {
        let results: Vec<VectorSearchResult> = driver
            .search_entities_by_vector(embedding, k, group_id)
            .await
            .map_err(|e| SemanticSearchError::DriverError(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|r| ScoredEntity {
                entity: r.entity,
                score: r.score,
                source: "semantic".to_string(),
            })
            .collect())
    }
}

/// A search result with a relevance score.
#[derive(Debug, Clone)]
pub struct ScoredEntity {
    /// The entity found.
    pub entity: crate::graph::Entity,
    /// Relevance score (0.0 - 1.0).
    pub score: f64,
    /// Which search method produced this result.
    pub source: String,
}

/// Errors from semantic search.
#[derive(Debug, thiserror::Error)]
pub enum SemanticSearchError {
    #[error("Embedder error: {0}")]
    EmbedderError(String),

    #[error("Driver error: {0}")]
    DriverError(String),
}
