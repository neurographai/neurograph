// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! LLM-based cross-encoder reranking.
//!
//! After hybrid retrieval returns candidates, the reranker uses an LLM
//! to score each candidate's relevance to the query. This produces
//! more accurate rankings at the cost of additional LLM calls.
//!
//! Influenced by Graphiti's cross-encoder reranking (cross_encoder/).

use crate::llm::traits::{CompletionRequest, LlmClient, LlmUsage};

use super::hybrid::HybridSearchResult;

/// Reranker configuration.
#[derive(Debug, Clone)]
pub struct RerankerConfig {
    /// Maximum number of candidates to rerank (controls cost).
    pub max_candidates: usize,
    /// Whether reranking is enabled.
    pub enabled: bool,
}

impl Default for RerankerConfig {
    fn default() -> Self {
        Self {
            max_candidates: 10,
            enabled: false, // Disabled by default to save cost
        }
    }
}

/// The cross-encoder reranker.
pub struct Reranker {
    config: RerankerConfig,
}

impl Reranker {
    /// Create a new reranker.
    pub fn new(config: RerankerConfig) -> Self {
        Self { config }
    }

    /// Rerank search results using an LLM as a cross-encoder.
    ///
    /// For each candidate, asks the LLM: "How relevant is this entity to the query?"
    /// The LLM returns a relevance score (0-10) which replaces the original score.
    pub async fn rerank(
        &self,
        query: &str,
        results: Vec<HybridSearchResult>,
        llm: &dyn LlmClient,
    ) -> Result<(Vec<HybridSearchResult>, LlmUsage), RerankerError> {
        if !self.config.enabled || results.is_empty() {
            return Ok((results, LlmUsage::default()));
        }

        let candidates: Vec<_> = results
            .into_iter()
            .take(self.config.max_candidates)
            .collect();

        // Build a batch reranking prompt
        let entities_text: Vec<String> = candidates
            .iter()
            .enumerate()
            .map(|(i, r)| {
                format!(
                    "{}. {} ({}): {}",
                    i + 1,
                    r.entity.name,
                    r.entity.entity_type,
                    if r.entity.summary.is_empty() {
                        "No description"
                    } else {
                        &r.entity.summary
                    }
                )
            })
            .collect();

        let prompt = format!(
            "Given the query: \"{}\"\n\n\
            Rate each entity's relevance on a scale of 0-10.\n\
            Respond with JSON: {{\"scores\": [score1, score2, ...]}}\n\n\
            Entities:\n{}",
            query,
            entities_text.join("\n")
        );

        let request = CompletionRequest::new(prompt)
            .with_system("You are a relevance scoring assistant. Rate each entity's relevance to the query. Respond only with JSON.")
            .with_json_mode()
            .with_temperature(0.0);

        let response = llm
            .complete(request)
            .await
            .map_err(|e| RerankerError::LlmError(e.to_string()))?;

        // Parse scores
        let scores = Self::parse_scores(&response.content, candidates.len());

        let mut reranked: Vec<HybridSearchResult> = candidates
            .into_iter()
            .enumerate()
            .map(|(i, mut r)| {
                if let Some(&score) = scores.get(i) {
                    r.score = score / 10.0; // Normalize to 0.0-1.0
                }
                r
            })
            .collect();

        reranked.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok((reranked, response.usage))
    }

    /// Parse scores from LLM JSON response.
    fn parse_scores(content: &str, expected: usize) -> Vec<f64> {
        #[derive(serde::Deserialize)]
        struct ScoresResponse {
            scores: Vec<f64>,
        }

        if let Ok(parsed) = serde_json::from_str::<ScoresResponse>(content) {
            return parsed.scores;
        }

        // Fallback: try to extract numbers from the response
        let mut scores = Vec::new();
        for word in content.split(|c: char| !c.is_ascii_digit() && c != '.') {
            if let Ok(score) = word.parse::<f64>() {
                if (0.0..=10.0).contains(&score) {
                    scores.push(score);
                }
            }
        }

        // Pad with default scores if needed
        while scores.len() < expected {
            scores.push(5.0); // Default middle score
        }

        scores.truncate(expected);
        scores
    }
}

/// Errors from reranking.
#[derive(Debug, thiserror::Error)]
pub enum RerankerError {
    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scores() {
        let scores = Reranker::parse_scores(r#"{"scores": [8.5, 6.0, 3.2]}"#, 3);
        assert_eq!(scores.len(), 3);
        assert!((scores[0] - 8.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_scores_fallback() {
        let scores = Reranker::parse_scores("Scores: 8, 6, 3", 3);
        assert_eq!(scores.len(), 3);
    }

    #[test]
    fn test_parse_scores_padding() {
        let scores = Reranker::parse_scores("", 3);
        assert_eq!(scores.len(), 3); // Should pad with defaults
    }
}
