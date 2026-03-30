// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Cross-encoder reranking for improving retrieval precision.
//!
//! After RRF fusion produces a candidate list, the cross-encoder
//! rescores each (query, candidate) pair using a more expensive
//! but more accurate model. Three backends:
//!
//! 1. **RuleBased** — Zero-cost offline reranker using keyword overlap,
//!    exact phrase matching, and entity-density heuristics.
//! 2. **Api** — Calls a remote reranking API (Cohere, Jina, etc.)
//! 3. **Onnx** — Local ONNX model (behind feature flag, e.g. BGE-reranker-v2-m3)
//!
//! Reference: "Multi-strategy retrieval with cross-encoder reranking" (2026).

use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Cross-encoder reranker that rescores RRF candidates.
pub struct CrossEncoderReranker {
    backend: RerankerBackend,
}

/// Reranker backend selection.
pub enum RerankerBackend {
    /// Zero-cost rule-based reranker (offline mode).
    RuleBased {
        /// Boost factor for keyword overlap (0.0–1.0).
        keyword_boost: f64,
        /// Boost factor for recency signals (0.0–1.0).
        recency_boost: f64,
    },
    /// API-based reranker (Cohere, Jina, etc.).
    Api {
        endpoint: String,
        api_key: String,
        model: String,
    },
}

/// A candidate document to be reranked.
#[derive(Debug, Clone)]
pub struct RerankCandidate {
    /// Document identifier.
    pub id: Uuid,
    /// Full text of the candidate.
    pub text: String,
    /// Score from the initial retrieval stage (pre-reranking).
    pub original_score: f64,
    /// Optional metadata for scoring heuristics.
    pub metadata: HashMap<String, String>,
}

/// Result from the cross-encoder reranking.
#[derive(Debug, Clone)]
pub struct RerankResult {
    /// Document identifier.
    pub id: Uuid,
    /// Document text.
    pub text: String,
    /// Score assigned by the cross-encoder.
    pub rerank_score: f64,
    /// Original retrieval score.
    pub original_score: f64,
    /// Weighted combination of rerank + original scores.
    pub combined_score: f64,
}

impl CrossEncoderReranker {
    /// Create a rule-based reranker (offline mode, zero cost).
    pub fn rule_based() -> Self {
        Self {
            backend: RerankerBackend::RuleBased {
                keyword_boost: 0.4,
                recency_boost: 0.1,
            },
        }
    }

    /// Create a rule-based reranker with custom weights.
    pub fn rule_based_with(keyword_boost: f64, recency_boost: f64) -> Self {
        Self {
            backend: RerankerBackend::RuleBased {
                keyword_boost,
                recency_boost,
            },
        }
    }

    /// Create an API-based reranker.
    pub fn api(endpoint: String, api_key: String, model: String) -> Self {
        Self {
            backend: RerankerBackend::Api {
                endpoint,
                api_key,
                model,
            },
        }
    }

    /// Rerank candidates given a query.
    ///
    /// Returns the top-k candidates sorted by combined score.
    pub async fn rerank(
        &self,
        query: &str,
        candidates: Vec<RerankCandidate>,
        top_k: usize,
    ) -> Result<Vec<RerankResult>, RerankerError> {
        match &self.backend {
            RerankerBackend::RuleBased {
                keyword_boost,
                recency_boost,
            } => Ok(self.rerank_rule_based(
                query,
                candidates,
                top_k,
                *keyword_boost,
                *recency_boost,
            )),
            RerankerBackend::Api {
                endpoint,
                api_key,
                model,
            } => {
                self.rerank_api(query, candidates, top_k, endpoint, api_key, model)
                    .await
            }
        }
    }

    /// Rule-based reranking using multiple heuristics.
    fn rerank_rule_based(
        &self,
        query: &str,
        candidates: Vec<RerankCandidate>,
        top_k: usize,
        keyword_boost: f64,
        _recency_boost: f64,
    ) -> Vec<RerankResult> {
        let query_lower = query.to_lowercase();
        let query_tokens: HashSet<String> = query_lower
            .split_whitespace()
            .filter(|s| s.len() > 1)
            .map(|s| s.to_string())
            .collect();

        let mut results: Vec<RerankResult> = candidates
            .into_iter()
            .map(|c| {
                let candidate_lower = c.text.to_lowercase();
                let candidate_tokens: HashSet<String> = candidate_lower
                    .split_whitespace()
                    .filter(|s| s.len() > 1)
                    .map(|s| s.to_string())
                    .collect();

                // 1. Keyword overlap ratio
                let overlap = query_tokens.intersection(&candidate_tokens).count() as f64;
                let max_tokens = query_tokens.len().max(1) as f64;
                let keyword_score = overlap / max_tokens;

                // 2. Exact phrase match bonus
                let phrase_bonus = if candidate_lower.contains(&query_lower) {
                    0.5
                } else {
                    // Check for partial phrases (bigrams from query)
                    let query_words: Vec<&str> = query_lower.split_whitespace().collect();
                    let mut bigram_matches: f64 = 0.0;
                    for window in query_words.windows(2) {
                        let bigram = format!("{} {}", window[0], window[1]);
                        if candidate_lower.contains(&bigram) {
                            bigram_matches += 0.15;
                        }
                    }
                    bigram_matches.min(0.3)
                };

                // 3. Entity density bonus — capitalized words per total words
                let total_words = c.text.split_whitespace().count().max(1) as f64;
                let cap_words = c
                    .text
                    .split_whitespace()
                    .filter(|w| {
                        w.chars()
                            .next()
                            .map(|ch| ch.is_uppercase())
                            .unwrap_or(false)
                    })
                    .count() as f64;
                let entity_density = (cap_words / total_words).min(0.5) * 0.2;

                let rerank_score = keyword_score * keyword_boost + phrase_bonus + entity_density;

                // Combine: 40% original score, 60% rerank score
                let combined = c.original_score * 0.4 + rerank_score * 0.6;

                RerankResult {
                    id: c.id,
                    text: c.text,
                    rerank_score,
                    original_score: c.original_score,
                    combined_score: combined,
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(top_k);
        results
    }

    /// API-based reranking via HTTP.
    async fn rerank_api(
        &self,
        query: &str,
        candidates: Vec<RerankCandidate>,
        top_k: usize,
        endpoint: &str,
        api_key: &str,
        model: &str,
    ) -> Result<Vec<RerankResult>, RerankerError> {
        let client = reqwest::Client::new();

        let documents: Vec<String> = candidates.iter().map(|c| c.text.clone()).collect();

        let body = serde_json::json!({
            "model": model,
            "query": query,
            "documents": documents,
            "top_n": top_k,
        });

        let response = client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| RerankerError::ApiError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RerankerError::ApiError(format!(
                "Reranker API returned status {}",
                response.status()
            )));
        }

        let resp_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| RerankerError::ApiError(e.to_string()))?;

        let mut results = Vec::new();
        if let Some(rankings) = resp_json.get("results").and_then(|r| r.as_array()) {
            for ranking in rankings {
                let index = ranking.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                let relevance_score = ranking
                    .get("relevance_score")
                    .and_then(|s| s.as_f64())
                    .unwrap_or(0.0);

                if index < candidates.len() {
                    let c = &candidates[index];
                    results.push(RerankResult {
                        id: c.id,
                        text: c.text.clone(),
                        rerank_score: relevance_score,
                        original_score: c.original_score,
                        combined_score: c.original_score * 0.3 + relevance_score * 0.7,
                    });
                }
            }
        }

        results.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(top_k);
        Ok(results)
    }
}

/// Errors from cross-encoder reranking.
#[derive(Debug, thiserror::Error)]
pub enum RerankerError {
    #[error("Reranker API error: {0}")]
    ApiError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rule_based_reranker() {
        let reranker = CrossEncoderReranker::rule_based();

        let candidates = vec![
            RerankCandidate {
                id: Uuid::new_v4(),
                text: "Alice works at Anthropic as a researcher".to_string(),
                original_score: 0.8,
                metadata: HashMap::new(),
            },
            RerankCandidate {
                id: Uuid::new_v4(),
                text: "Bob joined Google DeepMind last year".to_string(),
                original_score: 0.7,
                metadata: HashMap::new(),
            },
        ];

        let results = reranker
            .rerank("Alice Anthropic researcher", candidates, 10)
            .await
            .unwrap();

        assert!(!results.is_empty());
        assert!(
            results[0].text.contains("Alice"),
            "Alice doc should rank first"
        );
    }

    #[tokio::test]
    async fn test_exact_phrase_boost() {
        let reranker = CrossEncoderReranker::rule_based();

        let candidates = vec![
            RerankCandidate {
                id: Uuid::new_v4(),
                text: "working at Anthropic is great".to_string(),
                original_score: 0.5,
                metadata: HashMap::new(),
            },
            RerankCandidate {
                id: Uuid::new_v4(),
                text: "works at Anthropic".to_string(),
                original_score: 0.5,
                metadata: HashMap::new(),
            },
        ];

        let results = reranker
            .rerank("works at Anthropic", candidates, 10)
            .await
            .unwrap();

        // Exact phrase match should score higher
        assert_eq!(results.len(), 2);
        assert!(
            results[0].text.contains("works at Anthropic"),
            "Exact phrase match should rank higher"
        );
    }
}
