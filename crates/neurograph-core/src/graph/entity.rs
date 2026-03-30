// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Entity node representation.
//!
//! Influenced by Graphiti's `EntityNode` (nodes.py L492-620):
//! - uuid, name, name_embedding, summary, attributes, labels, created_at
//!   Enhanced with:
//! - `entity_type` for ontology classification (from Cognee)
//! - `importance_score` for PageRank-based decay (our innovation)
//! - `updated_at` for tracking modifications
//! - `metadata` for extensible key-value pairs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for an entity node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub Uuid);

impl EntityId {
    /// Create a new random entity ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from an existing UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the inner UUID as a string.
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Classification type for entities.
///
/// From Cognee's typed ontology system — entities have explicit types
/// that can be user-defined (prescribed) or auto-discovered (learned).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityType(pub String);

impl EntityType {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An entity node in the knowledge graph.
///
/// Entities represent real-world objects, people, places, concepts, etc.
/// Each entity has an optional embedding for semantic search and a summary
/// generated from its surrounding relationships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Unique identifier (UUID v4).
    pub id: EntityId,

    /// Human-readable name (e.g., "Alice", "Anthropic", "San Francisco").
    pub name: String,

    /// Classification type (e.g., "Person", "Organization", "Location").
    pub entity_type: EntityType,

    /// LLM-generated summary of this entity based on its relationships.
    /// Follows Graphiti's pattern where entity summaries are regional
    /// descriptions of the entity's role in the graph.
    pub summary: String,

    /// Embedding vector of the entity name for semantic search.
    /// Nullable because embeddings may not be computed yet.
    pub name_embedding: Option<Vec<f32>>,

    /// Group/partition identifier for multi-tenant graphs.
    /// From Graphiti's `group_id` pattern.
    pub group_id: String,

    /// Labels for flexible categorization (e.g., ["Entity", "Person"]).
    /// From Graphiti's label system.
    pub labels: Vec<String>,

    /// Extensible key-value attributes.
    /// From Graphiti's `attributes: dict[str, Any]`.
    pub attributes: HashMap<String, serde_json::Value>,

    /// Importance score (0.0 - 1.0) computed from PageRank + access frequency.
    /// Used by the forgetting/decay system to determine what to prune.
    pub importance_score: f64,

    /// Number of times this entity has been accessed in queries.
    pub access_count: u64,

    /// Community membership at each hierarchy level.
    /// Maps level → community_id.
    /// From GraphRAG's hierarchical community structure.
    pub community_ids: HashMap<u32, String>,

    /// When this entity was first created.
    pub created_at: DateTime<Utc>,

    /// When this entity was last modified.
    pub updated_at: DateTime<Utc>,

    /// Arbitrary metadata for extensions and plugins.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Entity {
    /// Create a new entity with the given name and type.
    pub fn new(name: impl Into<String>, entity_type: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: EntityId::new(),
            name: name.into(),
            entity_type: EntityType::new(entity_type),
            summary: String::new(),
            name_embedding: None,
            group_id: String::from("default"),
            labels: vec!["Entity".to_string()],
            attributes: HashMap::new(),
            importance_score: 0.5,
            access_count: 0,
            community_ids: HashMap::new(),
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Builder method to set the group ID.
    pub fn with_group_id(mut self, group_id: impl Into<String>) -> Self {
        self.group_id = group_id.into();
        self
    }

    /// Builder method to set the summary.
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }

    /// Builder method to add a label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }

    /// Builder method to set an attribute.
    pub fn with_attribute(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.attributes.insert(key.into(), value);
        self
    }

    /// Builder method to set the embedding.
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.name_embedding = Some(embedding);
        self
    }

    /// Update the modification timestamp.
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Record an access event (for importance scoring).
    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.touch();
    }
}

impl PartialEq for Entity {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Entity {}

impl std::hash::Hash for Entity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        let entity = Entity::new("Alice", "Person");
        assert_eq!(entity.name, "Alice");
        assert_eq!(entity.entity_type.as_str(), "Person");
        assert!(entity.labels.contains(&"Entity".to_string()));
        assert_eq!(entity.group_id, "default");
        assert!(entity.name_embedding.is_none());
    }

    #[test]
    fn test_entity_builder() {
        let entity = Entity::new("Anthropic", "Organization")
            .with_group_id("research")
            .with_summary("An AI safety company")
            .with_label("Company")
            .with_attribute("founded", serde_json::json!(2021));

        assert_eq!(entity.group_id, "research");
        assert_eq!(entity.summary, "An AI safety company");
        assert!(entity.labels.contains(&"Company".to_string()));
        assert_eq!(entity.attributes["founded"], serde_json::json!(2021));
    }

    #[test]
    fn test_entity_serialization() {
        let entity = Entity::new("Test", "TestType");
        let json = serde_json::to_string(&entity).unwrap();
        let deserialized: Entity = serde_json::from_str(&json).unwrap();
        assert_eq!(entity.id, deserialized.id);
        assert_eq!(entity.name, deserialized.name);
    }
}
