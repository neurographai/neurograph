// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Hybrid retrieval engine — 5-stage pipeline combining semantic, keyword,
//! PPR, graph traversal, and cross-encoder reranking.
//!
//! Uses Reciprocal Rank Fusion (RRF) to merge results from multiple
//! search methods into a single ranked list, then optionally reranks
//! with a cross-encoder for precision.
//!
//! Pipeline:
//!   1. Semantic search (vector similarity)
//!   2. BM25 keyword search
//!   3. Personalized PageRank (seeded from step 1)
//!   4. Graph traversal (BFS from seed entities)
//!   5. Cross-encoder reranking (rule-based or API)
//!
//! Influenced by Graphiti's hybrid search (search/search_utils.py)
//! and GraphRAG v3's multi-strategy retrieval.

use std::collections::HashMap;

use crate::drivers::traits::GraphDriver;
use crate::embedders::traits::Embedder;
use crate::graph::{Entity, EntityId};

use super::cross_encoder::{CrossEncoderReranker, RerankCandidate};
use super::keyword::KeywordSearcher;
use super::ppr::PersonalizedPageRank;
use super::semantic::{ScoredEntity, SemanticSearcher};
use super::traversal::TraversalSearcher;

/// Weights for combining different search methods in the 5-stage pipeline.
#[derive(Debug, Clone)]
pub struct RetrievalWeights {
    /// Weight for semantic (vector) search results.
    pub semantic: f64,
    /// Weight for keyword (BM25) search results.
    pub keyword: f64,
    /// Weight for graph traversal results.
    pub traversal: f64,
    /// Weight for Personalized PageRank results.
    pub ppr: f64,
}

impl Default for RetrievalWeights {
    fn default() -> Self {
        Self {
            semantic: 0.40,
            keyword: 0.25,
            traversal: 0.15,
            ppr: 0.20,
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

/// Per-strategy scoring breakdown for observability.
#[derive(Debug, Clone)]
pub struct StrategyBreakdown {
    /// Strategy name.
    pub strategy: String,
    /// Number of results from this strategy.
    pub result_count: usize,
    /// Latency for this strategy in milliseconds.
    pub latency_ms: u64,
}

/// Extended result with strategy breakdown.
#[derive(Debug)]
pub struct HybridSearchReport {
    /// Ranked search results.
    pub results: Vec<HybridSearchResult>,
    /// Per-strategy performance breakdown.
    pub strategies: Vec<StrategyBreakdown>,
    /// Whether cross-encoder reranking was applied.
    pub reranked: bool,
    /// Total pipeline latency in milliseconds.
    pub total_latency_ms: u64,
}

/// The hybrid retriever combining all search methods.
pub struct HybridRetriever {
    weights: RetrievalWeights,
    /// RRF constant (typically 60).
    rrf_k: f64,
    /// Optional cross-encoder reranker (stage 5).
    reranker: Option<CrossEncoderReranker>,
    /// Whether PPR is enabled.
    ppr_enabled: bool,
}

impl HybridRetriever {
    /// Create with default weights (no reranking).
    pub fn new() -> Self {
        Self {
            weights: RetrievalWeights::default(),
            rrf_k: 60.0,
            reranker: None,
            ppr_enabled: true,
        }
    }

    /// Create with custom weights.
    pub fn with_weights(weights: RetrievalWeights) -> Self {
        Self {
            weights,
            rrf_k: 60.0,
            reranker: None,
            ppr_enabled: true,
        }
    }

    /// Enable cross-encoder reranking (stage 5).
    pub fn with_reranker(mut self, reranker: CrossEncoderReranker) -> Self {
        self.reranker = Some(reranker);
        self
    }

    /// Enable rule-based reranking (zero-cost offline mode).
    pub fn with_rule_based_reranking(mut self) -> Self {
        self.reranker = Some(CrossEncoderReranker::rule_based());
        self
    }

    /// Disable PPR (for lightweight queries).
    pub fn without_ppr(mut self) -> Self {
        self.ppr_enabled = false;
        self
    }

    /// Execute the full 5-stage hybrid search pipeline.
    ///
    /// 1. Semantic search (vector similarity)
    /// 2. BM25 keyword search
    /// 3. Personalized PageRank (seeded from step 1)
    /// 4. Graph traversal (BFS from seed entities)
    /// 5. Cross-encoder reranking (if enabled)
    pub async fn search(
        &self,
        query: &str,
        k: usize,
        seed_ids: Option<&[EntityId]>,
        group_id: Option<&str>,
        embedder: &dyn Embedder,
        driver: &dyn GraphDriver,
    ) -> Result<Vec<HybridSearchResult>, HybridSearchError> {
        let report = self
            .search_with_report(query, k, seed_ids, group_id, embedder, driver)
            .await?;
        Ok(report.results)
    }

    /// Execute with detailed strategy breakdown.
    pub async fn search_with_report(
        &self,
        query: &str,
        k: usize,
        seed_ids: Option<&[EntityId]>,
        group_id: Option<&str>,
        embedder: &dyn Embedder,
        driver: &dyn GraphDriver,
    ) -> Result<HybridSearchReport, HybridSearchError> {
        let pipeline_start = std::time::Instant::now();
        let search_k = k * 3; // Fetch more candidates per method
        let mut strategies: Vec<StrategyBreakdown> = Vec::new();

        // Stage 1: Semantic search
        let t0 = std::time::Instant::now();
        let semantic_results =
            SemanticSearcher::search(query, search_k, group_id, embedder, driver)
                .await
                .unwrap_or_default();
        strategies.push(StrategyBreakdown {
            strategy: "semantic".to_string(),
            result_count: semantic_results.len(),
            latency_ms: t0.elapsed().as_millis() as u64,
        });

        // Stage 2: Keyword search
        let t1 = std::time::Instant::now();
        let keyword_results = KeywordSearcher::search(query, search_k, group_id, driver)
            .await
            .unwrap_or_default();
        strategies.push(StrategyBreakdown {
            strategy: "keyword".to_string(),
            result_count: keyword_results.len(),
            latency_ms: t1.elapsed().as_millis() as u64,
        });

        // Stage 3: Personalized PageRank (seeded from semantic results)
        let ppr_results = if self.ppr_enabled && !semantic_results.is_empty() {
            let t2 = std::time::Instant::now();
            let ppr_entities = self.run_ppr(&semantic_results, search_k, driver).await;
            strategies.push(StrategyBreakdown {
                strategy: "ppr".to_string(),
                result_count: ppr_entities.len(),
                latency_ms: t2.elapsed().as_millis() as u64,
            });
            ppr_entities
        } else {
            Vec::new()
        };

        // Stage 4: Graph traversal
        let t3 = std::time::Instant::now();
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
        strategies.push(StrategyBreakdown {
            strategy: "traversal".to_string(),
            result_count: traversal_results.len(),
            latency_ms: t3.elapsed().as_millis() as u64,
        });

        // RRF Fusion across all stages
        let fused = self.rrf_fuse(
            &semantic_results,
            &keyword_results,
            &ppr_results,
            &traversal_results,
        );

        let mut results: Vec<HybridSearchResult> = fused.into_values().collect();
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Stage 5: Cross-encoder reranking
        let reranked = if let Some(reranker) = &self.reranker {
            let t4 = std::time::Instant::now();
            let candidates: Vec<RerankCandidate> = results
                .iter()
                .map(|r| RerankCandidate {
                    id: uuid::Uuid::new_v4(), // Use entity summary as rerank context
                    text: format!(
                        "{} ({}): {}",
                        r.entity.name, r.entity.entity_type, r.entity.summary
                    ),
                    original_score: r.score,
                    metadata: HashMap::new(),
                })
                .collect();

            if let Ok(reranked_results) = reranker.rerank(query, candidates, k).await {
                // Map reranked scores back to entities
                let entity_map: HashMap<usize, &HybridSearchResult> =
                    results.iter().enumerate().map(|(i, r)| (i, r)).collect();

                results = reranked_results
                    .iter()
                    .enumerate()
                    .filter_map(|(i, rr)| {
                        entity_map.get(&i).map(|original| HybridSearchResult {
                            entity: original.entity.clone(),
                            score: rr.combined_score,
                            sources: {
                                let mut sources = original.sources.clone();
                                sources.push("reranked".to_string());
                                sources
                            },
                        })
                    })
                    .collect();

                strategies.push(StrategyBreakdown {
                    strategy: "cross_encoder".to_string(),
                    result_count: results.len(),
                    latency_ms: t4.elapsed().as_millis() as u64,
                });
                true
            } else {
                false
            }
        } else {
            false
        };

        results.truncate(k);

        Ok(HybridSearchReport {
            results,
            strategies,
            reranked,
            total_latency_ms: pipeline_start.elapsed().as_millis() as u64,
        })
    }

    /// Run PPR seeded from semantic search results.
    ///
    /// Builds a local adjacency subgraph from the seed entities' neighborhoods
    /// and runs Personalized PageRank to find structurally important entities.
    async fn run_ppr(
        &self,
        semantic_results: &[ScoredEntity],
        _k: usize,
        driver: &dyn GraphDriver,
    ) -> Vec<ScoredEntity> {
        if semantic_results.is_empty() {
            return Vec::new();
        }

        // Build adjacency from seed entities' relationships
        let mut adjacency: HashMap<uuid::Uuid, Vec<(uuid::Uuid, f64)>> = HashMap::new();
        let mut entity_cache: HashMap<uuid::Uuid, Entity> = HashMap::new();

        // Seed entities and their 1-hop neighborhoods
        for result in semantic_results.iter().take(5) {
            let uuid = result.entity.id.0;
            entity_cache.insert(uuid, result.entity.clone());

            if let Ok(rels) = driver.get_entity_relationships(&result.entity.id).await {
                for rel in &rels {
                    let src_uuid = rel.source_entity_id.0;
                    let tgt_uuid = rel.target_entity_id.0;

                    adjacency.entry(src_uuid).or_default().push((tgt_uuid, 1.0));

                    // Try to cache the connected entity
                    if !entity_cache.contains_key(&tgt_uuid) {
                        if let Ok(entity) = driver.get_entity(&rel.target_entity_id).await {
                            entity_cache.insert(tgt_uuid, entity);
                        }
                    }
                    if !entity_cache.contains_key(&src_uuid) {
                        if let Ok(entity) = driver.get_entity(&rel.source_entity_id).await {
                            entity_cache.insert(src_uuid, entity);
                        }
                    }
                }
            }
        }

        if adjacency.is_empty() {
            return Vec::new();
        }

        // Build seed map
        let mut seeds: HashMap<uuid::Uuid, f64> = HashMap::new();
        for result in semantic_results.iter().take(5) {
            seeds.insert(result.entity.id.0, result.score);
        }

        // Run PPR
        let ppr = PersonalizedPageRank::new(0.85, 20, 1e-8);
        let scores = ppr.compute(&adjacency, &seeds);

        // Map scores back to ScoredEntity (using cached entities)
        let mut ppr_results: Vec<ScoredEntity> = scores
            .into_iter()
            .filter_map(|(uuid, score)| {
                entity_cache.get(&uuid).map(|entity| ScoredEntity {
                    entity: entity.clone(),
                    score,
                    source: "ppr".to_string(),
                })
            })
            .collect();

        ppr_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ppr_results
    }

    /// 4-way Reciprocal Rank Fusion.
    fn rrf_fuse(
        &self,
        semantic: &[ScoredEntity],
        keyword: &[ScoredEntity],
        ppr: &[ScoredEntity],
        traversal: &[ScoredEntity],
    ) -> HashMap<String, HybridSearchResult> {
        let mut fused: HashMap<String, HybridSearchResult> = HashMap::new();

        let sources = [
            (semantic, self.weights.semantic, "semantic"),
            (keyword, self.weights.keyword, "keyword"),
            (ppr, self.weights.ppr, "ppr"),
            (traversal, self.weights.traversal, "traversal"),
        ];

        for (results, weight, source_name) in &sources {
            for (rank, result) in results.iter().enumerate() {
                let key = result.entity.id.as_str();
                let rrf_score = weight / (self.rrf_k + rank as f64 + 1.0);

                fused
                    .entry(key)
                    .and_modify(|existing| {
                        existing.score += rrf_score;
                        existing.sources.push(source_name.to_string());
                    })
                    .or_insert_with(|| HybridSearchResult {
                        entity: result.entity.clone(),
                        score: rrf_score,
                        sources: vec![source_name.to_string()],
                    });
            }
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

        let fused = retriever.rrf_fuse(&semantic, &keyword, &[], &[]);

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
        let total = weights.semantic + weights.keyword + weights.traversal + weights.ppr;
        assert!(
            (total - 1.0).abs() < f64::EPSILON,
            "Weights should sum to 1.0"
        );
    }

    #[test]
    fn test_ppr_weight_in_fusion() {
        let retriever = HybridRetriever::new();

        let entity_a = Entity::new("Alice", "Person");
        let entity_b = Entity::new("Bob", "Person");

        let semantic = vec![ScoredEntity {
            entity: entity_a.clone(),
            score: 0.9,
            source: "semantic".to_string(),
        }];

        let ppr = vec![ScoredEntity {
            entity: entity_b.clone(),
            score: 0.8,
            source: "ppr".to_string(),
        }];

        let fused = retriever.rrf_fuse(&semantic, &[], &ppr, &[]);
        assert!(fused.contains_key(&entity_a.id.as_str()));
        assert!(fused.contains_key(&entity_b.id.as_str()));
    }

    #[test]
    fn test_builder_with_reranker() {
        let retriever = HybridRetriever::new()
            .with_rule_based_reranking()
            .without_ppr();

        assert!(retriever.reranker.is_some());
        assert!(!retriever.ppr_enabled);
    }
}
