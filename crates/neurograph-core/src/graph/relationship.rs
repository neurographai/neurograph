// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Relationship (edge) representation with bi-temporal validity.
//!
//! Influenced by Graphiti's `EntityEdge` (edges.py L263-282):
//! - fact, fact_embedding, valid_at, invalid_at, expired_at, episodes
//!   Enhanced with:
//! - `weight` for weighted graph algorithms (community detection, PageRank)
//! - `confidence` for extraction quality scoring

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::entity::EntityId;
use super::episode::EpisodeId;

/// Unique identifier for a relationship edge.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelationshipId(pub Uuid);

impl RelationshipId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for RelationshipId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RelationshipId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A relationship edge in the temporal knowledge graph.
///
/// Relationships connect two entities and carry a temporal fact with
/// validity windows. The bi-temporal model comes directly from Graphiti:
/// - `valid_from`: When the fact became true in the real world
/// - `valid_until`: When the fact stopped being true (None = still valid)
/// - `created_at`: When we recorded this fact (system time)
/// - `expired_at`: When we invalidated this record (system time)
///
/// This allows point-in-time queries: "What was true on date X?"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Unique identifier.
    pub id: RelationshipId,

    /// Source entity UUID.
    pub source_entity_id: EntityId,

    /// Target entity UUID.
    pub target_entity_id: EntityId,

    /// Relationship type name (e.g., "WORKS_AT", "LIVES_IN", "KNOWS").
    pub relationship_type: String,

    /// Human-readable name/label for this edge.
    pub name: String,

    /// Natural language fact that this edge represents.
    /// e.g., "Alice works at Anthropic as a research scientist"
    pub fact: String,

    /// Embedding vector of the fact for semantic search.
    pub fact_embedding: Option<Vec<f32>>,

    /// Edge weight for graph algorithms (community detection, etc.).
    /// Default 1.0, adjusted by extraction confidence and access frequency.
    pub weight: f64,

    /// Confidence score from extraction (0.0 - 1.0).
    pub confidence: f64,

    /// Group/partition identifier for multi-tenant graphs.
    pub group_id: String,

    /// Episode IDs that reference this relationship.
    /// Provides provenance — which source documents mentioned this fact.
    pub episode_ids: Vec<EpisodeId>,

    // --- Bi-temporal fields (from Graphiti) ---
    /// When this fact became true in the real world.
    /// e.g., "March 2026" for "Alice moved to SF in March 2026".
    pub valid_from: Option<DateTime<Utc>>,

    /// When this fact stopped being true in the real world.
    /// None = still currently valid.
    /// e.g., Set when "Alice left SF" is ingested.
    pub valid_until: Option<DateTime<Utc>>,

    /// When this record was created in the system.
    pub created_at: DateTime<Utc>,

    /// When this record was invalidated/superseded in the system.
    /// Different from valid_until: this is about our knowledge, not reality.
    pub expired_at: Option<DateTime<Utc>>,

    /// Extensible key-value attributes.
    pub attributes: HashMap<String, serde_json::Value>,
}

impl Relationship {
    /// Create a new relationship between two entities.
    pub fn new(
        source: EntityId,
        target: EntityId,
        relationship_type: impl Into<String>,
        fact: impl Into<String>,
    ) -> Self {
        let rel_type = relationship_type.into();
        let now = Utc::now();
        Self {
            id: RelationshipId::new(),
            source_entity_id: source,
            target_entity_id: target,
            name: rel_type.clone(),
            relationship_type: rel_type,
            fact: fact.into(),
            fact_embedding: None,
            weight: 1.0,
            confidence: 1.0,
            group_id: String::from("default"),
            episode_ids: Vec::new(),
            valid_from: Some(now),
            valid_until: None,
            created_at: now,
            expired_at: None,
            attributes: HashMap::new(),
        }
    }

    /// Check if this relationship is currently valid (not invalidated).
    pub fn is_valid(&self) -> bool {
        self.expired_at.is_none() && self.valid_until.is_none()
    }

    /// Check if this relationship was valid at a specific point in time.
    pub fn is_valid_at(&self, timestamp: &DateTime<Utc>) -> bool {
        let after_start = self.valid_from.map(|vf| timestamp >= &vf).unwrap_or(true);

        let before_end = self.valid_until.map(|vu| timestamp < &vu).unwrap_or(true);

        after_start && before_end && self.expired_at.is_none()
    }

    /// Invalidate this relationship (temporal conflict resolution).
    /// Sets `valid_until` to the given timestamp and `expired_at` to now.
    pub fn invalidate(&mut self, valid_until: DateTime<Utc>) {
        self.valid_until = Some(valid_until);
        self.expired_at = Some(Utc::now());
    }

    /// Builder: set group ID.
    pub fn with_group_id(mut self, group_id: impl Into<String>) -> Self {
        self.group_id = group_id.into();
        self
    }

    /// Builder: set weight.
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// Builder: set confidence.
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    /// Builder: set valid_from.
    pub fn with_valid_from(mut self, valid_from: DateTime<Utc>) -> Self {
        self.valid_from = Some(valid_from);
        self
    }

    /// Builder: set embedding.
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.fact_embedding = Some(embedding);
        self
    }

    /// Builder: add episode reference.
    pub fn with_episode(mut self, episode_id: EpisodeId) -> Self {
        self.episode_ids.push(episode_id);
        self
    }
}

impl PartialEq for Relationship {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Relationship {}

impl std::hash::Hash for Relationship {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationship_creation() {
        let src = EntityId::new();
        let tgt = EntityId::new();
        let rel = Relationship::new(
            src.clone(),
            tgt.clone(),
            "WORKS_AT",
            "Alice works at Anthropic",
        );

        assert_eq!(rel.source_entity_id, src);
        assert_eq!(rel.target_entity_id, tgt);
        assert_eq!(rel.relationship_type, "WORKS_AT");
        assert!(rel.is_valid());
    }

    #[test]
    fn test_temporal_validity() {
        let src = EntityId::new();
        let tgt = EntityId::new();
        let mut rel = Relationship::new(src, tgt, "LIVES_IN", "Alice lives in NYC");

        let past = Utc::now() - chrono::Duration::days(365);
        let future = Utc::now() + chrono::Duration::days(365);

        rel.valid_from = Some(past);
        assert!(rel.is_valid());
        assert!(rel.is_valid_at(&Utc::now()));

        // Invalidate
        rel.invalidate(Utc::now());
        assert!(!rel.is_valid());
        assert!(!rel.is_valid_at(&future));
    }
}
