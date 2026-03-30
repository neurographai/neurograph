// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Entity deduplication engine.
//!
//! Two-phase deduplication inspired by Graphiti's `utils/maintenance/` module:
//! - **Phase 1 (Deterministic)**: Name normalization + embedding cosine similarity
//! - **Phase 2 (LLM fallback)**: Ambiguous matches sent to LLM for resolution
//!
//! This prevents duplicate entities like "Alice" and "alice" or
//! "Anthropic" and "Anthropic Inc." from cluttering the graph.


use crate::drivers::traits::{GraphDriver, VectorSearchResult};
use crate::embedders::traits::Embedder;
use crate::graph::{Entity, EntityId};
use crate::utils::text::normalize_name;

/// Configuration for deduplication behavior.
#[derive(Debug, Clone)]
pub struct DeduplicationConfig {
    /// Cosine similarity threshold for automatic merge (Phase 1).
    /// Above this threshold, entities are merged without LLM confirmation.
    pub exact_threshold: f64,

    /// Cosine similarity threshold for LLM disambiguation (Phase 2).
    /// Between ambiguous_threshold and exact_threshold, we ask the LLM.
    /// Below ambiguous_threshold, entities are considered distinct.
    pub ambiguous_threshold: f64,

    /// Maximum number of candidates to consider for deduplication.
    pub max_candidates: usize,
}

impl Default for DeduplicationConfig {
    fn default() -> Self {
        Self {
            exact_threshold: 0.92,
            ambiguous_threshold: 0.75,
            max_candidates: 10,
        }
    }
}

/// Result of a deduplication check.
#[derive(Debug)]
pub enum DeduplicationResult {
    /// This is a new entity — no duplicates found.
    New,
    /// This entity matches an existing one — merge into the existing entity.
    Merge {
        /// The existing entity to merge into.
        existing_id: EntityId,
        /// Similarity score that triggered the merge.
        similarity: f64,
    },
}

/// The deduplication engine.
pub struct Deduplicator {
    config: DeduplicationConfig,
}

impl Deduplicator {
    /// Create a new deduplicator with default config.
    pub fn new() -> Self {
        Self {
            config: DeduplicationConfig::default(),
        }
    }

    /// Create with custom config.
    pub fn with_config(config: DeduplicationConfig) -> Self {
        Self { config }
    }

    /// Check if a new entity is a duplicate of an existing one.
    ///
    /// Phase 1: Name normalization + vector similarity
    ///   - Normalize both names (lowercase, trim, collapse whitespace)
    ///   - If normalized names match exactly → merge
    ///   - If embedding cosine similarity > exact_threshold → merge
    ///
    /// Phase 2: LLM disambiguation (future)
    ///   - If similarity is between ambiguous_threshold and exact_threshold
    ///   - Ask LLM: "Are these the same entity?"
    pub async fn check_duplicate(
        &self,
        entity_name: &str,
        entity_type: &str,
        embedding: Option<&[f32]>,
        driver: &dyn GraphDriver,
        embedder: &dyn Embedder,
    ) -> Result<DeduplicationResult, DeduplicationError> {
        let normalized = normalize_name(entity_name);

        // Phase 1a: Exact name match (after normalization)
        let existing = driver
            .search_entities_by_text(&normalized, self.config.max_candidates, None)
            .await
            .map_err(|e| DeduplicationError::DriverError(e.to_string()))?;

        for result in &existing {
            let existing_normalized = normalize_name(&result.entity.name);
            if existing_normalized == normalized {
                tracing::debug!(
                    entity = entity_name,
                    existing = %result.entity.name,
                    "Exact name match after normalization"
                );
                return Ok(DeduplicationResult::Merge {
                    existing_id: result.entity.id.clone(),
                    similarity: 1.0,
                });
            }
        }

        // Phase 1b: Vector similarity search
        let query_embedding = if let Some(emb) = embedding {
            emb.to_vec()
        } else {
            embedder
                .embed_one(entity_name)
                .await
                .map_err(|e| DeduplicationError::EmbedderError(e.to_string()))?
        };

        let vector_results: Vec<VectorSearchResult> = driver
            .search_entities_by_vector(
                &query_embedding,
                self.config.max_candidates,
                None,
            )
            .await
            .map_err(|e| DeduplicationError::DriverError(e.to_string()))?;

        for result in &vector_results {
            if result.score >= self.config.exact_threshold {
                // High similarity + same type = merge
                if result.entity.entity_type.as_str() == entity_type || entity_type == "Entity" {
                    tracing::debug!(
                        entity = entity_name,
                        existing = %result.entity.name,
                        score = result.score,
                        "Vector similarity merge"
                    );
                    return Ok(DeduplicationResult::Merge {
                        existing_id: result.entity.id.clone(),
                        similarity: result.score,
                    });
                }
            }

            // Phase 2: Ambiguous range (between thresholds)
            // Currently we treat ambiguous as "new entity" — LLM disambiguation
            // will be added in a future sprint to resolve these cases.
            if result.score >= self.config.ambiguous_threshold
                && result.score < self.config.exact_threshold
            {
                tracing::debug!(
                    entity = entity_name,
                    existing = %result.entity.name,
                    score = result.score,
                    "Ambiguous match — treating as new (LLM disambiguation TBD)"
                );
            }
        }

        Ok(DeduplicationResult::New)
    }

    /// Merge entity data from a new extraction into an existing entity.
    ///
    /// Updates the existing entity with any new information from the new entity,
    /// keeping the existing ID and name but enriching attributes and summary.
    pub fn merge_entities(existing: &mut Entity, new_name: &str, new_summary: &str) {
        // Update summary if the new one is more detailed
        if !new_summary.is_empty()
            && (existing.summary.is_empty() || new_summary.len() > existing.summary.len())
        {
            existing.summary = new_summary.to_string();
        }

        // Update timestamp
        existing.touch();

        tracing::debug!(
            existing_name = %existing.name,
            new_name = new_name,
            "Merged entity data"
        );
    }
}

impl Default for Deduplicator {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors during deduplication.
#[derive(Debug, thiserror::Error)]
pub enum DeduplicationError {
    #[error("Driver error: {0}")]
    DriverError(String),

    #[error("Embedder error: {0}")]
    EmbedderError(String),

    #[error("LLM disambiguation error: {0}")]
    LlmError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deduplication_config_defaults() {
        let config = DeduplicationConfig::default();
        assert!(config.exact_threshold > config.ambiguous_threshold);
        assert!(config.max_candidates > 0);
    }

    #[test]
    fn test_merge_entities() {
        let mut existing = Entity::new("Alice", "Person");
        existing.summary = "A person".to_string();

        Deduplicator::merge_entities(&mut existing, "Alice Johnson", "Alice Johnson is a researcher at Anthropic");

        assert_eq!(existing.summary, "Alice Johnson is a researcher at Anthropic");
    }

    #[test]
    fn test_merge_keeps_short_summary_if_no_new() {
        let mut existing = Entity::new("Alice", "Person");
        existing.summary = "A person".to_string();

        Deduplicator::merge_entities(&mut existing, "Alice", "");

        assert_eq!(existing.summary, "A person");
    }
}
