// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Hybrid retrieval engine combining semantic, keyword, and graph traversal.
//!
//! Uses Reciprocal Rank Fusion (RRF) to merge results from multiple
//! search methods into a single ranked list.
//!
//! Influenced by Graphiti's hybrid search (search/search_utils.py)
//! which combines vector, BM25, and graph traversal results.

use std::collections::HashMap;

use crate::drivers::traits::GraphDriver;
use crate::embedders::traits::Embedder;
use crate::graph::{Entity, EntityId};

use super::keyword::KeywordSearcher;
use super::semantic::{ScoredEntity, SemanticSearcher};
use super::traversal::TraversalSearcher;

/// Weights for combining different search methods.
#[derive(Debug, Clone)]
pub struct RetrievalWeights {
    /// Weight for semantic (vector) search results.
    pub semantic: f64,
    /// Weight for keyword (BM25) search results.
    pub keyword: f64,
    /// Weight for graph traversal results.
    pub traversal: f64,
}

impl Default for RetrievalWeights {
    fn default() -> Self {
        Self {
            semantic: 0.5,
            keyword: 0.3,
            traversal: 0.2,
        }
    }
}

/// A hybrid search result with fused score.
#[derive(Debug, Clone)]
pub struct HybridSearchResult {
    /// The entity found.
    pub entity: Entity,
    /// Fused relevance score.
    pub score: f64,
    /// Which search methods contributed to this result.
    pub sources: Vec<String>,
}

/// The hybrid retriever combining all three search methods.
pub struct HybridRetriever {
    weights: RetrievalWeights,
    /// RRF constant (typically 60).
    rrf_k: f64,
}

impl HybridRetriever {
    /// Create with default weights.
    pub fn new() -> Self {
        Self {
            weights: RetrievalWeights::default(),
            rrf_k: 60.0,
        }
    }

    /// Create with custom weights.
    pub fn with_weights(weights: RetrievalWeights) -> Self {
        Self {
            weights,
            rrf_k: 60.0,
        }
    }

    /// Execute a hybrid search combining semantic, keyword, and traversal.
    ///
    /// 1. Run all three search methods in parallel (conceptually)
    /// 2. Fuse results using Reciprocal Rank Fusion (RRF):
    ///    `score(d) = Σ weight_i / (rrf_k + rank_i(d))`
    /// 3. Sort by fused score, return top-k
    pub async fn search(
        &self,
        query: &str,
        k: usize,
        seed_ids: Option<&[EntityId]>,
        group_id: Option<&str>,
        embedder: &dyn Embedder,
        driver: &dyn GraphDriver,
    ) -> Result<Vec<HybridSearchResult>, HybridSearchError> {
        let search_k = k * 3; // Fetch more candidates per method

        // Run semantic search
        let semantic_results = SemanticSearcher::search(query, search_k, group_id, embedder, driver)
            .await
            .unwrap_or_default();

        // Run keyword search
        let keyword_results = KeywordSearcher::search(query, search_k, group_id, driver)
            .await
            .unwrap_or_default();

        // Run traversal search (only if we have seed entities)
        let traversal_results = if let Some(seeds) = seed_ids {
            if !seeds.is_empty() {
                TraversalSearcher::search(seeds, 2, search_k, group_id, driver)
                    .await
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // Reciprocal Rank Fusion
        let fused = self.rrf_fuse(
            &semantic_results,
            &keyword_results,
            &traversal_results,
        );

        // Take top-k
        let mut results: Vec<HybridSearchResult> = fused.into_values().collect();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);

        Ok(results)
    }

    /// Reciprocal Rank Fusion: merges ranked lists from multiple sources.
    ///
    /// For each document d and source i:
    ///   rrf_score(d) += weight_i / (rrf_k + rank_i(d))
    ///
    /// This is a standard information retrieval technique that handles
    /// different score scales gracefully.
    fn rrf_fuse(
        &self,
        semantic: &[ScoredEntity],
        keyword: &[ScoredEntity],
        traversal: &[ScoredEntity],
    ) -> HashMap<String, HybridSearchResult> {
        let mut fused: HashMap<String, HybridSearchResult> = HashMap::new();

        // Process semantic results
        for (rank, result) in semantic.iter().enumerate() {
            let key = result.entity.id.as_str();
            let rrf_score = self.weights.semantic / (self.rrf_k + rank as f64 + 1.0);

            fused
                .entry(key)
                .and_modify(|existing| {
                    existing.score += rrf_score;
                    existing.sources.push("semantic".to_string());
                })
                .or_insert_with(|| HybridSearchResult {
                    entity: result.entity.clone(),
                    score: rrf_score,
                    sources: vec!["semantic".to_string()],
                });
        }

        // Process keyword results
        for (rank, result) in keyword.iter().enumerate() {
            let key = result.entity.id.as_str();
            let rrf_score = self.weights.keyword / (self.rrf_k + rank as f64 + 1.0);

            fused
                .entry(key)
                .and_modify(|existing| {
                    existing.score += rrf_score;
                    existing.sources.push("keyword".to_string());
                })
                .or_insert_with(|| HybridSearchResult {
                    entity: result.entity.clone(),
                    score: rrf_score,
                    sources: vec!["keyword".to_string()],
                });
        }

        // Process traversal results
        for (rank, result) in traversal.iter().enumerate() {
            let key = result.entity.id.as_str();
            let rrf_score = self.weights.traversal / (self.rrf_k + rank as f64 + 1.0);

            fused
                .entry(key)
                .and_modify(|existing| {
                    existing.score += rrf_score;
                    existing.sources.push("traversal".to_string());
                })
                .or_insert_with(|| HybridSearchResult {
                    entity: result.entity.clone(),
                    score: rrf_score,
                    sources: vec!["traversal".to_string()],
                });
        }

        fused
    }
}

impl Default for HybridRetriever {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors from hybrid search.
#[derive(Debug, thiserror::Error)]
pub enum HybridSearchError {
    #[error("Search failed: {0}")]
    SearchFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Entity;

    #[test]
    fn test_rrf_fusion() {
        let retriever = HybridRetriever::new();

        let entity_a = Entity::new("Alice", "Person");
        let entity_b = Entity::new("Bob", "Person");

        let semantic = vec![
            ScoredEntity {
                entity: entity_a.clone(),
                score: 0.9,
                source: "semantic".to_string(),
            },
            ScoredEntity {
                entity: entity_b.clone(),
                score: 0.8,
                source: "semantic".to_string(),
            },
        ];

        let keyword = vec![ScoredEntity {
            entity: entity_a.clone(),
            score: 0.7,
            source: "keyword".to_string(),
        }];

        let fused = retriever.rrf_fuse(&semantic, &keyword, &[]);

        // Alice should have higher score (appears in both semantic and keyword)
        let alice_key = entity_a.id.as_str();
        let bob_key = entity_b.id.as_str();

        assert!(
            fused[&alice_key].score > fused[&bob_key].score,
            "Alice (in both lists) should score higher than Bob (semantic only)"
        );

        // Alice should have both sources
        assert!(fused[&alice_key].sources.contains(&"semantic".to_string()));
        assert!(fused[&alice_key].sources.contains(&"keyword".to_string()));
    }

    #[test]
    fn test_default_weights() {
        let weights = RetrievalWeights::default();
        let total = weights.semantic + weights.keyword + weights.traversal;
        assert!((total - 1.0).abs() < f64::EPSILON, "Weights should sum to 1.0");
    }
}
