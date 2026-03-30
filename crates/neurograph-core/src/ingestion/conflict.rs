// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Temporal conflict resolution.
//!
//! When a new fact contradicts an existing one, we don't delete the old fact.
//! Instead, we invalidate it by setting `valid_until` and `expired_at`, then
//! store the new fact with `valid_from = now`.
//!
//! This preserves full history and enables point-in-time queries.
//!
//! Influenced directly by Graphiti's `EntityEdge` temporal model
//! (edges.py L263-282: valid_at, invalid_at, expired_at).

use chrono::{DateTime, Utc};

use crate::drivers::traits::GraphDriver;
use crate::graph::{EntityId, Relationship};

/// A detected conflict between a new fact and an existing one.
#[derive(Debug)]
pub struct ConflictDetection {
    /// The existing relationship that conflicts.
    pub existing_relationship: Relationship,
    /// The type of conflict detected.
    pub conflict_type: ConflictType,
    /// Similarity score between the facts (if computed).
    pub similarity: f64,
}

/// Types of temporal conflicts.
#[derive(Debug, Clone)]
pub enum ConflictType {
    /// Direct contradiction: "Alice lives in NYC" vs "Alice lives in SF"
    /// Same source, same relationship type, different target.
    Contradiction,
    /// Supersession: new fact updates/replaces old fact
    /// e.g., "Alice's salary is $100k" → "Alice's salary is $120k"
    Supersession,
    /// Redundancy: new fact is the same as existing (skip it)
    Redundant,
}

/// The conflict resolver.
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
            // Only check relationships of the same type
            if rel.relationship_type != relationship_type {
                continue;
            }

            // Skip already-invalidated relationships
            if !rel.is_valid() {
                continue;
            }

            // Same target → redundant
            if rel.target_entity_id == *target_entity_id {
                conflicts.push(ConflictDetection {
                    existing_relationship: rel,
                    conflict_type: ConflictType::Redundant,
                    similarity: 1.0,
                });
                continue;
            }

            // Different target with same relationship type → contradiction
            // e.g., "Alice LIVES_IN NYC" vs "Alice LIVES_IN SF"
            conflicts.push(ConflictDetection {
                existing_relationship: rel,
                conflict_type: ConflictType::Contradiction,
                similarity: 0.0,
            });
        }

        Ok(conflicts)
    }

    /// Resolve a conflict by invalidating the old relationship.
    ///
    /// Sets `valid_until = now` and `expired_at = now` on the old relationship,
    /// then returns it for storage update. The new relationship should be stored
    /// separately with `valid_from = now`.
    pub fn resolve_contradiction(
        existing: &mut Relationship,
        resolution_time: DateTime<Utc>,
    ) {
        existing.invalidate(resolution_time);

        tracing::info!(
            relationship_id = %existing.id,
            fact = %existing.fact,
            valid_until = %resolution_time,
            "Invalidated contradicted relationship"
        );
    }
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
}
