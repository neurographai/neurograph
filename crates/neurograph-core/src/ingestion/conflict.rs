// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Temporal conflict resolution with schema-aware detection.
//!
//! When a new fact contradicts an existing one, we don't delete the old fact.
//! Instead, we invalidate it by setting `valid_until` and `expired_at`, then
//! store the new fact with `valid_from = now`.
//!
//! ## Enhancements over basic conflict detection
//!
//! 1. **Schema-Aware**: Distinguishes *exclusive* relations (WORKS_AT, LIVES_IN)
//!    from *non-exclusive* ones (LIKES, KNOWS). Only exclusive relations
//!    generate contradictions when targets differ.
//!
//! 2. **Semantic Similarity**: Uses embedding cosine similarity for fuzzy conflict
//!    detection — paraphrased facts ("Alice works at Anthropic" vs "Alice is
//!    employed by Anthropic") are detected as redundant rather than additive.
//!
//! 3. **Extended Resolution Strategies**: TemporalSupersession, Coexistence,
//!    ManualReview, and Unresolved.
//!
//! Influenced directly by Graphiti's `EntityEdge` temporal model
//! (edges.py L263-282: valid_at, invalid_at, expired_at).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::drivers::traits::GraphDriver;
use crate::graph::{EntityId, Relationship};

// ─── Relation Schema Registry ───────────────────────────────

/// Whether a relationship type is exclusive (only one active at a time)
/// or non-exclusive (multiple can coexist).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationCardinality {
    /// Only one active relationship of this type per source entity.
    /// e.g., WORKS_AT, LIVES_IN, MARRIED_TO — adding a new target invalidates the old.
    Exclusive,
    /// Multiple active relationships of this type per source entity.
    /// e.g., KNOWS, LIKES, HAS_SKILL — adding a new target coexists with existing.
    NonExclusive,
}

/// Registry of relation schemas that governs conflict detection behavior.
///
/// Relations not in the registry default to `NonExclusive` (safe default:
/// don't invalidate facts we're uncertain about).
#[derive(Debug, Clone)]
pub struct RelationSchemaRegistry {
    /// Relation types known to be exclusive.
    exclusive_types: HashSet<String>,
}

impl RelationSchemaRegistry {
    /// Create a new registry with common exclusive relation types.
    pub fn new() -> Self {
        let mut exclusive = HashSet::new();
        // Common exclusive relations where only one can be active
        for rel in &[
            "WORKS_AT",
            "EMPLOYED_BY",
            "LIVES_IN",
            "RESIDES_IN",
            "MARRIED_TO",
            "CEO_OF",
            "PRESIDENT_OF",
            "LOCATED_IN",
            "CAPITAL_OF",
            "REPORTS_TO",
            "MANAGES",
            "HEAD_OF",
            "STUDIES_AT",
            "ENROLLED_IN",
            "BORN_IN",
        ] {
            exclusive.insert(rel.to_string());
        }
        Self {
            exclusive_types: exclusive,
        }
    }

    /// Create an empty registry (all relations treated as non-exclusive).
    pub fn empty() -> Self {
        Self {
            exclusive_types: HashSet::new(),
        }
    }

    /// Register a relation type as exclusive.
    pub fn register_exclusive(&mut self, relation_type: impl Into<String>) {
        self.exclusive_types.insert(relation_type.into());
    }

    /// Register a relation type as non-exclusive (removes from exclusive set).
    pub fn register_non_exclusive(&mut self, relation_type: &str) {
        self.exclusive_types.remove(relation_type);
    }

    /// Get the cardinality of a relation type.
    pub fn cardinality(&self, relation_type: &str) -> RelationCardinality {
        // Normalize to uppercase for comparison
        let normalized = relation_type.to_uppercase();
        if self.exclusive_types.contains(&normalized) {
            RelationCardinality::Exclusive
        } else {
            RelationCardinality::NonExclusive
        }
    }
}

impl Default for RelationSchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Conflict Detection ─────────────────────────────────────

/// A detected conflict between a new fact and an existing one.
#[derive(Debug)]
pub struct ConflictDetection {
    /// The existing relationship that conflicts.
    pub existing_relationship: Relationship,
    /// The type of conflict detected.
    pub conflict_type: ConflictType,
    /// Semantic similarity score between the facts (0.0–1.0).
    pub similarity: f64,
}

/// Types of temporal conflicts.
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictType {
    /// Direct contradiction: exclusive relation with different target.
    /// e.g., "Alice LIVES_IN NYC" vs "Alice LIVES_IN SF"
    Contradiction,
    /// Supersession: semantically similar facts on the same exclusive relation.
    /// e.g., "Alice's salary is $100k" → "Alice's salary is $120k"
    Supersession,
    /// Redundancy: new fact is semantically identical to an existing one.
    Redundant,
    /// Soft conflict: non-exclusive relation with semantic overlap, may need review.
    SoftConflict,
}

/// How a conflict should be resolved.
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictResolution {
    /// Invalidate the old fact: set `valid_until = now`, store new fact.
    /// Used for exclusive relation contradictions with high confidence.
    TemporalSupersession,
    /// Both facts can coexist (non-exclusive relations).
    Coexistence,
    /// Cannot auto-resolve; flag for manual review.
    ManualReview { reason: String },
    /// Skip the new fact entirely (exact duplicate).
    Skip,
}

// ─── Conflict Detector ──────────────────────────────────────

/// Schema-aware, semantically-enhanced conflict detector.
///
/// Upgrades the basic `ConflictResolver` with:
/// - Relation schema awareness (exclusive vs non-exclusive)
/// - Embedding-based semantic similarity for fuzzy matching
/// - Extended resolution strategy selection
pub struct ConflictDetector {
    /// Relation schema registry.
    pub schema: RelationSchemaRegistry,
    /// Similarity threshold above which facts are considered semantically identical.
    pub redundancy_threshold: f64,
    /// Similarity threshold above which non-exclusive facts are flagged as soft conflicts.
    pub soft_conflict_threshold: f64,
}

impl ConflictDetector {
    /// Create a new detector with default schema and thresholds.
    pub fn new() -> Self {
        Self {
            schema: RelationSchemaRegistry::new(),
            redundancy_threshold: 0.92,
            soft_conflict_threshold: 0.75,
        }
    }

    /// Create with a custom schema registry.
    pub fn with_schema(schema: RelationSchemaRegistry) -> Self {
        Self {
            schema,
            redundancy_threshold: 0.92,
            soft_conflict_threshold: 0.75,
        }
    }

    /// Detect conflicts with schema awareness and optional semantic similarity.
    ///
    /// Enhancement over `ConflictResolver::detect_conflicts`:
    /// - Checks `RelationSchemaRegistry` to determine if relation is exclusive
    /// - Computes cosine similarity between fact embeddings when available
    /// - Produces richer conflict types (including `SoftConflict`)
    pub async fn detect_conflicts_enhanced(
        &self,
        source_entity_id: &EntityId,
        relationship_type: &str,
        target_entity_id: &EntityId,
        new_fact_embedding: Option<&[f32]>,
        driver: &dyn GraphDriver,
    ) -> Result<Vec<ConflictDetection>, ConflictError> {
        let existing = driver
            .get_entity_relationships(source_entity_id)
            .await
            .map_err(|e| ConflictError::DriverError(e.to_string()))?;

        let cardinality = self.schema.cardinality(relationship_type);
        let mut conflicts = Vec::new();

        for rel in existing {
            // Only check relationships of the same type
            if rel.relationship_type != relationship_type {
                continue;
            }

            // Skip already-invalidated relationships
            if !rel.is_valid() {
                continue;
            }

            // Compute semantic similarity if embeddings available
            let similarity = match (new_fact_embedding, rel.fact_embedding.as_deref()) {
                (Some(new_emb), Some(existing_emb)) => {
                    cosine_similarity(new_emb, existing_emb)
                }
                _ => 0.0, // No embeddings → fall back to structural check
            };

            // High similarity → redundant regardless of cardinality
            if similarity >= self.redundancy_threshold {
                conflicts.push(ConflictDetection {
                    existing_relationship: rel,
                    conflict_type: ConflictType::Redundant,
                    similarity,
                });
                continue;
            }

            // Same target → redundant (structural match)
            if rel.target_entity_id == *target_entity_id {
                conflicts.push(ConflictDetection {
                    existing_relationship: rel,
                    conflict_type: ConflictType::Redundant,
                    similarity: 1.0,
                });
                continue;
            }

            // Different target — behavior depends on cardinality
            match cardinality {
                RelationCardinality::Exclusive => {
                    // Exclusive relation + different target = contradiction
                    // If embeddings are somewhat similar, it's a supersession
                    let conflict_type = if similarity >= self.soft_conflict_threshold {
                        ConflictType::Supersession
                    } else {
                        ConflictType::Contradiction
                    };

                    conflicts.push(ConflictDetection {
                        existing_relationship: rel,
                        conflict_type,
                        similarity,
                    });
                }
                RelationCardinality::NonExclusive => {
                    // Non-exclusive: only flag if semantically very similar
                    if similarity >= self.soft_conflict_threshold {
                        conflicts.push(ConflictDetection {
                            existing_relationship: rel,
                            conflict_type: ConflictType::SoftConflict,
                            similarity,
                        });
                    }
                    // Otherwise: no conflict, facts coexist naturally
                }
            }
        }

        Ok(conflicts)
    }

    /// Determine the best resolution strategy for a conflict.
    pub fn resolve_strategy(&self, conflict: &ConflictDetection) -> ConflictResolution {
        match &conflict.conflict_type {
            ConflictType::Redundant => ConflictResolution::Skip,
            ConflictType::Contradiction => ConflictResolution::TemporalSupersession,
            ConflictType::Supersession => ConflictResolution::TemporalSupersession,
            ConflictType::SoftConflict => {
                if conflict.similarity >= self.redundancy_threshold {
                    ConflictResolution::Skip
                } else {
                    ConflictResolution::ManualReview {
                        reason: format!(
                            "Non-exclusive relation with {:.0}% semantic overlap",
                            conflict.similarity * 100.0
                        ),
                    }
                }
            }
        }
    }
}

impl Default for ConflictDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Legacy ConflictResolver (backwards compatibility) ──────

/// The original conflict resolver (preserved for backwards compatibility).
///
/// For new code, prefer `ConflictDetector` which provides schema awareness
/// and semantic similarity detection.
pub struct ConflictResolver;

impl ConflictResolver {
    /// Check if a new relationship conflicts with existing relationships.
    ///
    /// Conflict detection logic:
    /// 1. Get all existing relationships of the same type from the source entity
    /// 2. For each existing relationship:
    ///    a. If same source + same type + same target → Redundant (skip)
    ///    b. If same source + same type + different target → Contradiction
    ///    c. If same source + same type + overlapping fact → Supersession
    pub async fn detect_conflicts(
        source_entity_id: &EntityId,
        relationship_type: &str,
        target_entity_id: &EntityId,
        driver: &dyn GraphDriver,
    ) -> Result<Vec<ConflictDetection>, ConflictError> {
        let existing = driver
            .get_entity_relationships(source_entity_id)
            .await
            .map_err(|e| ConflictError::DriverError(e.to_string()))?;

        let mut conflicts = Vec::new();

        for rel in existing {
            if rel.relationship_type != relationship_type {
                continue;
            }

            if !rel.is_valid() {
                continue;
            }

            if rel.target_entity_id == *target_entity_id {
                conflicts.push(ConflictDetection {
                    existing_relationship: rel,
                    conflict_type: ConflictType::Redundant,
                    similarity: 1.0,
                });
                continue;
            }

            conflicts.push(ConflictDetection {
                existing_relationship: rel,
                conflict_type: ConflictType::Contradiction,
                similarity: 0.0,
            });
        }

        Ok(conflicts)
    }

    /// Resolve a conflict by invalidating the old relationship.
    pub fn resolve_contradiction(existing: &mut Relationship, resolution_time: DateTime<Utc>) {
        existing.invalidate(resolution_time);

        tracing::info!(
            relationship_id = %existing.id,
            fact = %existing.fact,
            valid_until = %resolution_time,
            "Invalidated contradicted relationship"
        );
    }
}

// ─── Helpers ─────────────────────────────────────────────────

/// Cosine similarity between two embedding vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    (dot / (norm_a * norm_b)) as f64
}

/// Errors during conflict resolution.
#[derive(Debug, thiserror::Error)]
pub enum ConflictError {
    #[error("Driver error: {0}")]
    DriverError(String),

    #[error("Resolution failed: {0}")]
    ResolutionFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::entity::EntityId;

    // --- Legacy ConflictResolver tests ---

    #[test]
    fn test_resolve_contradiction() {
        let src = EntityId::new();
        let tgt = EntityId::new();
        let mut rel = Relationship::new(src, tgt, "LIVES_IN", "Alice lives in NYC");

        assert!(rel.is_valid());

        let now = Utc::now();
        ConflictResolver::resolve_contradiction(&mut rel, now);

        assert!(!rel.is_valid());
        assert_eq!(rel.valid_until, Some(now));
        assert!(rel.expired_at.is_some());
    }

    // --- RelationSchemaRegistry tests ---

    #[test]
    fn test_default_schema_exclusive_types() {
        let schema = RelationSchemaRegistry::new();
        assert_eq!(schema.cardinality("WORKS_AT"), RelationCardinality::Exclusive);
        assert_eq!(schema.cardinality("LIVES_IN"), RelationCardinality::Exclusive);
        assert_eq!(schema.cardinality("MARRIED_TO"), RelationCardinality::Exclusive);
    }

    #[test]
    fn test_default_schema_non_exclusive_types() {
        let schema = RelationSchemaRegistry::new();
        // Unknown types default to non-exclusive
        assert_eq!(schema.cardinality("KNOWS"), RelationCardinality::NonExclusive);
        assert_eq!(schema.cardinality("LIKES"), RelationCardinality::NonExclusive);
        assert_eq!(schema.cardinality("HAS_SKILL"), RelationCardinality::NonExclusive);
    }

    #[test]
    fn test_schema_case_insensitive() {
        let schema = RelationSchemaRegistry::new();
        assert_eq!(schema.cardinality("works_at"), RelationCardinality::Exclusive);
        assert_eq!(schema.cardinality("Works_At"), RelationCardinality::Exclusive);
    }

    #[test]
    fn test_custom_schema_registration() {
        let mut schema = RelationSchemaRegistry::empty();
        assert_eq!(
            schema.cardinality("WORKS_AT"),
            RelationCardinality::NonExclusive
        );

        schema.register_exclusive("WORKS_AT");
        assert_eq!(schema.cardinality("WORKS_AT"), RelationCardinality::Exclusive);

        schema.register_non_exclusive("WORKS_AT");
        assert_eq!(
            schema.cardinality("WORKS_AT"),
            RelationCardinality::NonExclusive
        );
    }

    // --- ConflictDetector tests ---

    #[test]
    fn test_resolve_strategy_contradiction() {
        let detector = ConflictDetector::new();
        let src = EntityId::new();
        let tgt = EntityId::new();
        let rel = Relationship::new(src, tgt, "LIVES_IN", "Alice lives in NYC");

        let conflict = ConflictDetection {
            existing_relationship: rel,
            conflict_type: ConflictType::Contradiction,
            similarity: 0.0,
        };

        assert_eq!(
            detector.resolve_strategy(&conflict),
            ConflictResolution::TemporalSupersession
        );
    }

    #[test]
    fn test_resolve_strategy_redundant() {
        let detector = ConflictDetector::new();
        let src = EntityId::new();
        let tgt = EntityId::new();
        let rel = Relationship::new(src, tgt, "LIVES_IN", "Alice lives in NYC");

        let conflict = ConflictDetection {
            existing_relationship: rel,
            conflict_type: ConflictType::Redundant,
            similarity: 1.0,
        };

        assert_eq!(detector.resolve_strategy(&conflict), ConflictResolution::Skip);
    }

    #[test]
    fn test_resolve_strategy_soft_conflict() {
        let detector = ConflictDetector::new();
        let src = EntityId::new();
        let tgt = EntityId::new();
        let rel = Relationship::new(src, tgt, "KNOWS", "Alice knows Bob");

        let conflict = ConflictDetection {
            existing_relationship: rel,
            conflict_type: ConflictType::SoftConflict,
            similarity: 0.80,
        };

        let resolution = detector.resolve_strategy(&conflict);
        assert!(matches!(resolution, ConflictResolution::ManualReview { .. }));
    }

    // --- Cosine similarity tests ---

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }
}
