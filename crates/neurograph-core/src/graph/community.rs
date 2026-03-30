// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Community detection result representation.
//!
//! Influenced by GraphRAG's community structure:
//!   `Communities = list[tuple[level, cluster_id, parent_cluster, [node_ids]]]`
//!   from `cluster_graph.py` L14
//!
//! And Graphiti's `CommunityNode` (nodes.py L674-800):
//!   uuid, name, name_embedding, summary, group_id

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::entity::EntityId;

/// Unique identifier for a community.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommunityId(pub String);

impl CommunityId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CommunityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Community hierarchy level.
///
/// From GraphRAG's hierarchical Leiden: communities exist at multiple
/// resolution levels. Level 0 = finest, Level N = coarsest.
pub type CommunityLevel = u32;

/// A community of related entities detected by Leiden/Louvain.
///
/// Communities group related entities and have LLM-generated summaries
/// that enable global queries over the entire dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Community {
    /// Unique identifier for this community.
    pub id: CommunityId,

    /// Human-readable name (often LLM-generated).
    pub name: String,

    /// Hierarchy level (0 = finest granularity).
    /// From GraphRAG's multi-level community structure.
    pub level: CommunityLevel,

    /// Parent community ID in the hierarchy.
    /// From GraphRAG's `parent_cluster` field.
    pub parent_id: Option<CommunityId>,

    /// Child community IDs (for traversing the hierarchy).
    pub children_ids: Vec<CommunityId>,

    /// Member entity IDs.
    /// From GraphRAG's `list[node_ids]` in the Communities tuple.
    pub member_entity_ids: Vec<EntityId>,

    /// LLM-generated summary of this community.
    /// From GraphRAG's community summarization (map-reduce pattern).
    pub summary: String,

    /// Embedding of the community name/summary for search.
    pub name_embedding: Option<Vec<f32>>,

    /// Group/partition identifier.
    pub group_id: String,

    /// Whether this community needs re-summarization.
    /// Used by the incremental update system.
    pub is_dirty: bool,

    /// When this community was first detected.
    pub created_at: DateTime<Utc>,

    /// When this community was last updated.
    pub updated_at: DateTime<Utc>,

    /// Metadata for extensions.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Community {
    /// Create a new community at the given level.
    pub fn new(id: impl Into<String>, level: CommunityLevel) -> Self {
        let now = Utc::now();
        Self {
            id: CommunityId::new(id),
            name: String::new(),
            level,
            parent_id: None,
            children_ids: Vec::new(),
            member_entity_ids: Vec::new(),
            summary: String::new(),
            name_embedding: None,
            group_id: String::from("default"),
            is_dirty: true,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Add a member entity to this community.
    pub fn add_member(&mut self, entity_id: EntityId) {
        if !self.member_entity_ids.contains(&entity_id) {
            self.member_entity_ids.push(entity_id);
            self.is_dirty = true;
            self.updated_at = Utc::now();
        }
    }

    /// Remove a member entity from this community.
    pub fn remove_member(&mut self, entity_id: &EntityId) {
        self.member_entity_ids.retain(|id| id != entity_id);
        self.is_dirty = true;
        self.updated_at = Utc::now();
    }

    /// Set the summary and mark as clean.
    pub fn set_summary(&mut self, summary: impl Into<String>) {
        self.summary = summary.into();
        self.is_dirty = false;
        self.updated_at = Utc::now();
    }

    /// Get an iterator over member entity IDs.
    pub fn members(&self) -> impl Iterator<Item = &EntityId> {
        self.member_entity_ids.iter()
    }

    /// Get the number of members.
    pub fn member_count(&self) -> usize {
        self.member_entity_ids.len()
    }

    /// Builder: set parent community.
    pub fn with_parent(mut self, parent_id: CommunityId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Builder: set name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Builder: set group ID.
    pub fn with_group_id(mut self, group_id: impl Into<String>) -> Self {
        self.group_id = group_id.into();
        self
    }
}

impl PartialEq for Community {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Community {}

impl std::hash::Hash for Community {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_community_creation() {
        let community = Community::new("c1", 0);
        assert_eq!(community.level, 0);
        assert!(community.is_dirty);
        assert!(community.member_entity_ids.is_empty());
    }

    #[test]
    fn test_community_members() {
        let mut community = Community::new("c1", 0);
        let entity_id = EntityId::new();

        community.add_member(entity_id.clone());
        assert_eq!(community.member_count(), 1);

        // Adding same member twice should not duplicate
        community.add_member(entity_id.clone());
        assert_eq!(community.member_count(), 1);

        community.remove_member(&entity_id);
        assert_eq!(community.member_count(), 0);
    }

    #[test]
    fn test_community_hierarchy() {
        let parent = Community::new("parent", 1);
        let child = Community::new("child", 0)
            .with_parent(parent.id.clone());
        assert_eq!(child.parent_id.as_ref().unwrap().as_str(), "parent");
    }
}
