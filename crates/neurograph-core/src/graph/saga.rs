// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Saga (conversation thread) representation.
//!
//! A Saga groups Episodes into an ordered conversation thread, enabling:
//! - Chronological replay of conversation/document sequences
//! - "What was discussed in conversation X?" queries
//! - Ordered provenance chains for multi-turn interactions
//!
//! Closes a critical competitive gap — Graphiti has `SagaNode` with
//! `HAS_EPISODE`/`NEXT_EPISODE` edges for threading conversations.
//!
//! # Graph Structure
//!
//! ```text
//! Saga ──[HAS_EPISODE {position: 0}]──> Episode 1
//!   │                                       │
//!   ├──[HAS_EPISODE {position: 1}]──> Episode 2
//!   │                                       │
//!   └──[HAS_EPISODE {position: 2}]──> Episode 3
//!
//! Episode 1 ──[NEXT_EPISODE]──> Episode 2 ──[NEXT_EPISODE]──> Episode 3
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for a saga.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SagaId(pub Uuid);

impl SagaId {
    /// Create a new random saga ID.
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

impl Default for SagaId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SagaId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A Saga groups Episodes into an ordered conversation thread.
///
/// Analogous to Graphiti's `SagaNode` — tracks a single conversation,
/// document ingestion session, or interaction sequence. Sagas provide
/// ordering semantics (via `episode_ids`) and group identity.
///
/// # Examples
///
/// ```rust
/// use neurograph_core::graph::saga::Saga;
///
/// let mut saga = Saga::new("Research Session: Transformer Architectures");
/// saga.set_source("chat_session_42");
/// saga.set_description("Multi-turn discussion about attention mechanisms");
///
/// // Episodes are added via NeuroGraph::add_episode_to_saga()
/// // which handles both the saga update and edge creation.
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Saga {
    /// Unique identifier.
    pub id: SagaId,

    /// Human-readable name for this saga.
    pub name: String,

    /// Optional description of the saga's topic or purpose.
    pub description: Option<String>,

    /// Source identifier (e.g., "chat_session_42", "paper_arxiv_2401.12345").
    pub source: Option<String>,

    /// Group/partition identifier for multi-tenant graphs.
    pub group_id: String,

    /// Ordered list of episode IDs in this saga.
    /// Maintains insertion order (append-only).
    pub episode_ids: Vec<String>,

    /// When this saga was created.
    pub created_at: DateTime<Utc>,

    /// When this saga was last updated (episode added/modified).
    pub updated_at: DateTime<Utc>,

    /// Arbitrary metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Saga {
    /// Create a new empty saga.
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: SagaId::new(),
            name: name.into(),
            description: None,
            source: None,
            group_id: String::from("default"),
            episode_ids: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Create a saga with a specific group.
    pub fn with_group(mut self, group_id: impl Into<String>) -> Self {
        self.group_id = group_id.into();
        self
    }

    /// Set the description.
    pub fn set_description(&mut self, desc: impl Into<String>) {
        self.description = Some(desc.into());
        self.updated_at = Utc::now();
    }

    /// Set the source identifier.
    pub fn set_source(&mut self, source: impl Into<String>) {
        self.source = Some(source.into());
        self.updated_at = Utc::now();
    }

    /// Append an episode to this saga, maintaining chronological order.
    ///
    /// Returns the position (0-indexed) of the newly added episode.
    pub fn add_episode(&mut self, episode_id: impl Into<String>) -> usize {
        let position = self.episode_ids.len();
        self.episode_ids.push(episode_id.into());
        self.updated_at = Utc::now();
        position
    }

    /// Get consecutive episode pairs for NEXT_EPISODE edges.
    ///
    /// Returns `(source_episode_id, target_episode_id)` pairs.
    pub fn episode_pairs(&self) -> Vec<(&str, &str)> {
        self.episode_ids
            .windows(2)
            .map(|w| (w[0].as_str(), w[1].as_str()))
            .collect()
    }

    /// Get the number of episodes in this saga.
    pub fn episode_count(&self) -> usize {
        self.episode_ids.len()
    }

    /// Get the most recent episode ID.
    pub fn latest_episode(&self) -> Option<&str> {
        self.episode_ids.last().map(|s| s.as_str())
    }

    /// Get the first episode ID.
    pub fn first_episode(&self) -> Option<&str> {
        self.episode_ids.first().map(|s| s.as_str())
    }

    /// Check if this saga contains a specific episode.
    pub fn contains_episode(&self, episode_id: &str) -> bool {
        self.episode_ids.iter().any(|id| id == episode_id)
    }

    /// Get the position of an episode in this saga.
    pub fn episode_position(&self, episode_id: &str) -> Option<usize> {
        self.episode_ids.iter().position(|id| id == episode_id)
    }

    /// Check if the saga is empty (no episodes).
    pub fn is_empty(&self) -> bool {
        self.episode_ids.is_empty()
    }

    /// Get saga duration (from first to last episode creation, if tracked externally).
    /// Returns None if there are fewer than 2 episodes.
    pub fn has_sequence(&self) -> bool {
        self.episode_ids.len() >= 2
    }
}

/// Well-known edge types for saga threading.
pub mod edge_types {
    /// Edge from Saga to Episode: `Saga --[HAS_EPISODE]--> Episode`
    pub const HAS_EPISODE: &str = "HAS_EPISODE";

    /// Edge between consecutive episodes: `Episode --[NEXT_EPISODE]--> Episode`
    pub const NEXT_EPISODE: &str = "NEXT_EPISODE";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_saga_creation() {
        let saga = Saga::new("Test Conversation");
        assert_eq!(saga.name, "Test Conversation");
        assert!(saga.is_empty());
        assert_eq!(saga.episode_count(), 0);
        assert!(saga.latest_episode().is_none());
    }

    #[test]
    fn test_add_episodes() {
        let mut saga = Saga::new("Test");

        let pos0 = saga.add_episode("ep_1");
        let pos1 = saga.add_episode("ep_2");
        let pos2 = saga.add_episode("ep_3");

        assert_eq!(pos0, 0);
        assert_eq!(pos1, 1);
        assert_eq!(pos2, 2);
        assert_eq!(saga.episode_count(), 3);
        assert_eq!(saga.latest_episode(), Some("ep_3"));
        assert_eq!(saga.first_episode(), Some("ep_1"));
    }

    #[test]
    fn test_episode_pairs() {
        let mut saga = Saga::new("Test");
        saga.add_episode("ep_1");
        saga.add_episode("ep_2");
        saga.add_episode("ep_3");

        let pairs = saga.episode_pairs();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("ep_1", "ep_2"));
        assert_eq!(pairs[1], ("ep_2", "ep_3"));
    }

    #[test]
    fn test_episode_pairs_single() {
        let mut saga = Saga::new("Test");
        saga.add_episode("ep_1");

        let pairs = saga.episode_pairs();
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_contains_and_position() {
        let mut saga = Saga::new("Test");
        saga.add_episode("ep_1");
        saga.add_episode("ep_2");

        assert!(saga.contains_episode("ep_1"));
        assert!(!saga.contains_episode("ep_99"));
        assert_eq!(saga.episode_position("ep_2"), Some(1));
        assert_eq!(saga.episode_position("ep_99"), None);
    }

    #[test]
    fn test_with_group() {
        let saga = Saga::new("Test").with_group("tenant_42");
        assert_eq!(saga.group_id, "tenant_42");
    }

    #[test]
    fn test_setters() {
        let mut saga = Saga::new("Test");
        saga.set_description("A test saga");
        saga.set_source("unit_test");

        assert_eq!(saga.description, Some("A test saga".to_string()));
        assert_eq!(saga.source, Some("unit_test".to_string()));
    }

    #[test]
    fn test_has_sequence() {
        let mut saga = Saga::new("Test");
        assert!(!saga.has_sequence());
        saga.add_episode("ep_1");
        assert!(!saga.has_sequence());
        saga.add_episode("ep_2");
        assert!(saga.has_sequence());
    }
}
