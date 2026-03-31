// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Maximal Marginal Relevance (MMR) reranker.
//!
//! Balances relevance to query with diversity among selected results.
//! Closes a critical competitive gap — both Graphiti and GraphRAG
//! support MMR-based reranking.
//!
//! Formula:
//!   MMR(d) = λ · sim(d, q) - (1-λ) · max_{d_j ∈ S} sim(d, d_j)
//!
//! Where:
//!   - λ = 1.0 → pure relevance (identical to top-k by score)
//!   - λ = 0.0 → pure diversity (maximally different from selected)
//!   - λ = 0.5 → balanced trade-off (recommended default)

use std::collections::HashSet;

/// The similarity metric used for inter-document comparison.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SimilarityMetric {
    /// Cosine similarity (default, normalized dot product).
    Cosine,
    /// Raw dot product (for pre-normalized embeddings).
    DotProduct,
}

impl Default for SimilarityMetric {
    fn default() -> Self {
        Self::Cosine
    }
}

/// Maximal Marginal Relevance reranker.
///
/// Greedily selects items that balance relevance to the query with
/// diversity among already-selected items. This prevents redundant
/// results when multiple candidates cover the same topic.
///
/// # Example
///
/// ```rust
/// use neurograph_core::retrieval::mmr::MmrReranker;
///
/// let mmr = MmrReranker::new(0.7); // 70% relevance, 30% diversity
///
/// let query_emb = vec![1.0, 0.0, 0.0];
/// let candidates = vec![
///     ("doc_a", vec![0.9, 0.1, 0.0], 0.95),  // very relevant
///     ("doc_b", vec![0.85, 0.15, 0.0], 0.90), // relevant but similar to a
///     ("doc_c", vec![0.1, 0.9, 0.0], 0.60),   // less relevant but diverse
/// ];
///
/// let selected = mmr.rerank(&query_emb, &candidates, 2);
/// // doc_a selected first (highest relevance),
/// // doc_c may beat doc_b (diversity bonus outweighs relevance gap)
/// ```
#[derive(Debug, Clone)]
pub struct MmrReranker {
    /// Trade-off: 1.0 = pure relevance, 0.0 = pure diversity.
    lambda: f32,
    /// Similarity function for inter-document comparison.
    similarity: SimilarityMetric,
}

impl MmrReranker {
    /// Create a new MMR reranker with the given lambda trade-off.
    ///
    /// - `lambda = 1.0`: pure relevance (no diversity)
    /// - `lambda = 0.7`: recommended default (slight diversity)
    /// - `lambda = 0.5`: balanced relevance and diversity
    /// - `lambda = 0.0`: pure diversity (no relevance)
    pub fn new(lambda: f32) -> Self {
        Self {
            lambda: lambda.clamp(0.0, 1.0),
            similarity: SimilarityMetric::Cosine,
        }
    }

    /// Use a custom similarity metric.
    pub fn with_similarity(mut self, metric: SimilarityMetric) -> Self {
        self.similarity = metric;
        self
    }

    /// Get the lambda (relevance-diversity trade-off) value.
    pub fn lambda(&self) -> f32 {
        self.lambda
    }

    /// Greedily select `k` items from `candidates` using MMR.
    ///
    /// # Arguments
    /// - `query_embedding`: the query vector
    /// - `candidates`: `(item, embedding, relevance_score)` triples
    /// - `k`: number of items to select
    ///
    /// # Returns
    /// Selected items with MMR scores, in selection order.
    pub fn rerank<T: Clone>(
        &self,
        query_embedding: &[f32],
        candidates: &[(T, Vec<f32>, f32)],
        k: usize,
    ) -> Vec<(T, f32)> {
        if candidates.is_empty() {
            return Vec::new();
        }

        let k = k.min(candidates.len());
        let mut selected: Vec<usize> = Vec::with_capacity(k);
        let mut selected_set: HashSet<usize> = HashSet::with_capacity(k);
        let mut results: Vec<(T, f32)> = Vec::with_capacity(k);

        // Precompute query similarities for all candidates
        let query_sims: Vec<f32> = candidates
            .iter()
            .map(|(_, emb, _)| self.sim(query_embedding, emb))
            .collect();

        for _ in 0..k {
            let mut best_idx = None;
            let mut best_mmr = f32::NEG_INFINITY;

            for (i, _) in candidates.iter().enumerate() {
                if selected_set.contains(&i) {
                    continue;
                }

                let relevance = query_sims[i];

                // Max similarity to any already-selected item
                let max_sim_to_selected = if selected.is_empty() {
                    0.0
                } else {
                    selected
                        .iter()
                        .map(|&j| self.sim(&candidates[i].1, &candidates[j].1))
                        .fold(f32::NEG_INFINITY, f32::max)
                };

                let mmr =
                    self.lambda * relevance - (1.0 - self.lambda) * max_sim_to_selected;

                if mmr > best_mmr {
                    best_mmr = mmr;
                    best_idx = Some(i);
                }
            }

            match best_idx {
                Some(idx) => {
                    selected.push(idx);
                    selected_set.insert(idx);
                    results.push((candidates[idx].0.clone(), best_mmr));
                }
                None => break,
            }
        }

        results
    }

    /// Rerank items that already have embeddings and scores attached.
    /// Convenience wrapper around `rerank` for `MmrCandidate` inputs.
    pub fn rerank_candidates(
        &self,
        query_embedding: &[f32],
        candidates: &[MmrCandidate],
        k: usize,
    ) -> Vec<MmrResult> {
        let triples: Vec<(usize, Vec<f32>, f32)> = candidates
            .iter()
            .enumerate()
            .map(|(i, c)| (i, c.embedding.clone(), c.score))
            .collect();

        self.rerank(query_embedding, &triples, k)
            .into_iter()
            .map(|(idx, mmr_score)| MmrResult {
                original_index: idx,
                mmr_score,
                original_score: candidates[idx].score,
            })
            .collect()
    }

    /// Compute similarity between two vectors.
    fn sim(&self, a: &[f32], b: &[f32]) -> f32 {
        match self.similarity {
            SimilarityMetric::Cosine => cosine_similarity(a, b),
            SimilarityMetric::DotProduct => dot_product(a, b),
        }
    }
}

impl Default for MmrReranker {
    fn default() -> Self {
        Self::new(0.7)
    }
}

/// Input candidate for MMR reranking.
#[derive(Debug, Clone)]
pub struct MmrCandidate {
    /// Embedding vector for this candidate.
    pub embedding: Vec<f32>,
    /// Original relevance score (e.g., from cosine search).
    pub score: f32,
}

/// Output result from MMR reranking.
#[derive(Debug, Clone)]
pub struct MmrResult {
    /// Index into the original candidates array.
    pub original_index: usize,
    /// MMR score (combines relevance and diversity).
    pub mmr_score: f32,
    /// Original relevance score before MMR.
    pub original_score: f32,
}

/// Compute the dot product of two vectors.
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = dot_product(a, b);
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mmr_pure_relevance() {
        // λ = 1.0: pure relevance, should return in order of query similarity
        let mmr = MmrReranker::new(1.0);

        let query = vec![1.0, 0.0, 0.0];
        let candidates = vec![
            ("best", vec![0.95, 0.05, 0.0], 0.95),
            ("second", vec![0.8, 0.2, 0.0], 0.80),
            ("third", vec![0.5, 0.5, 0.0], 0.50),
        ];

        let selected = mmr.rerank(&query, &candidates, 3);
        assert_eq!(selected.len(), 3);
        assert_eq!(selected[0].0, "best");
        assert_eq!(selected[1].0, "second");
        assert_eq!(selected[2].0, "third");
    }

    #[test]
    fn test_mmr_promotes_diversity() {
        // λ = 0.3: strong diversity preference
        let mmr = MmrReranker::new(0.3);

        let query = vec![1.0, 0.0, 0.0];
        let candidates = vec![
            ("relevant_a", vec![0.95, 0.05, 0.0], 0.95),
            // Very similar to relevant_a (should be penalized)
            ("clone_of_a", vec![0.94, 0.06, 0.0], 0.94),
            // Less relevant but diverse (should be promoted)
            ("diverse_b", vec![0.1, 0.9, 0.1], 0.40),
        ];

        let selected = mmr.rerank(&query, &candidates, 2);
        assert_eq!(selected.len(), 2);
        // First pick: relevant_a (highest relevance)
        assert_eq!(selected[0].0, "relevant_a");
        // Second pick: diverse_b (clone_of_a penalized for similarity to a)
        assert_eq!(selected[1].0, "diverse_b");
    }

    #[test]
    fn test_mmr_empty_candidates() {
        let mmr = MmrReranker::new(0.5);
        let query = vec![1.0, 0.0];
        let candidates: Vec<(&str, Vec<f32>, f32)> = vec![];
        let selected = mmr.rerank(&query, &candidates, 5);
        assert!(selected.is_empty());
    }

    #[test]
    fn test_mmr_k_larger_than_candidates() {
        let mmr = MmrReranker::new(0.5);
        let query = vec![1.0, 0.0];
        let candidates = vec![
            ("only", vec![0.9, 0.1], 0.9),
        ];
        let selected = mmr.rerank(&query, &candidates, 10);
        assert_eq!(selected.len(), 1);
    }

    #[test]
    fn test_mmr_candidates_api() {
        let mmr = MmrReranker::new(0.7);
        let query = vec![1.0, 0.0];
        let candidates = vec![
            MmrCandidate { embedding: vec![0.9, 0.1], score: 0.9 },
            MmrCandidate { embedding: vec![0.1, 0.9], score: 0.4 },
        ];
        let results = mmr.rerank_candidates(&query, &candidates, 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].original_index, 0);
    }

    #[test]
    fn test_cosine_similarity_unit_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-6);

        let c = vec![1.0, 0.0];
        assert!((cosine_similarity(&a, &c) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let a = vec![0.0, 0.0];
        let b = vec![1.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_dot_product() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        assert!((dot_product(&a, &b) - 32.0).abs() < 1e-6);
    }

    #[test]
    fn test_default_lambda() {
        let mmr = MmrReranker::default();
        assert!((mmr.lambda() - 0.7).abs() < f32::EPSILON);
    }
}
