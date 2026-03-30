// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Episode (provenance) node representation.
//!
//! Influenced by Graphiti's `EpisodicNode` (nodes.py L315-460):
//! - source, source_description, content, valid_at, entity_edges
//!
//! Episodes are the provenance layer — they record WHERE knowledge came from.
//! Every entity and relationship traces back to one or more episodes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for an episode.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EpisodeId(pub Uuid);

impl EpisodeId {
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

impl Default for EpisodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EpisodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type of data source for an episode.
///
/// From Graphiti's `EpisodeType` enum (nodes.py L54-86).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EpisodeType {
    /// Chat message (formatted as "actor: content").
    Message,
    /// Structured JSON data.
    Json,
    /// Plain text document.
    Text,
    /// Image file (behind multimodal feature flag).
    Image,
    /// Audio file (behind multimodal feature flag).
    Audio,
    /// PDF document (behind multimodal feature flag).
    Pdf,
}

impl std::fmt::Display for EpisodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EpisodeType::Message => write!(f, "message"),
            EpisodeType::Json => write!(f, "json"),
            EpisodeType::Text => write!(f, "text"),
            EpisodeType::Image => write!(f, "image"),
            EpisodeType::Audio => write!(f, "audio"),
            EpisodeType::Pdf => write!(f, "pdf"),
        }
    }
}

/// An episode node — the provenance/source data record.
///
/// Every piece of knowledge in the graph traces back to an episode.
/// Episodes are immutable once created (append-only provenance).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// Unique identifier.
    pub id: EpisodeId,

    /// Human-readable name for this episode.
    pub name: String,

    /// Type of source data.
    pub source_type: EpisodeType,

    /// Description of where this data came from.
    /// e.g., "User conversation", "Wikipedia article", "API response"
    pub source_description: String,

    /// Raw content of the episode.
    /// For messages: "user: Hello, how are you?"
    /// For JSON: the raw JSON string
    /// For text: the raw text
    pub content: String,

    /// Group/partition identifier for multi-tenant graphs.
    pub group_id: String,

    /// Relationship edge UUIDs that were extracted from this episode.
    pub entity_edge_ids: Vec<String>,

    /// When the original source document was created/valid.
    /// This is the "real world" timestamp of the data.
    pub valid_at: DateTime<Utc>,

    /// When this episode was ingested into the system.
    pub created_at: DateTime<Utc>,

    /// Arbitrary metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Episode {
    /// Create a new text episode.
    pub fn text(name: impl Into<String>, content: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: EpisodeId::new(),
            name: name.into(),
            source_type: EpisodeType::Text,
            source_description: String::new(),
            content: content.into(),
            group_id: String::from("default"),
            entity_edge_ids: Vec::new(),
            valid_at: now,
            created_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Create a new message episode.
    pub fn message(name: impl Into<String>, content: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: EpisodeId::new(),
            name: name.into(),
            source_type: EpisodeType::Message,
            source_description: String::new(),
            content: content.into(),
            group_id: String::from("default"),
            entity_edge_ids: Vec::new(),
            valid_at: now,
            created_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Create a new JSON episode.
    pub fn json(name: impl Into<String>, content: serde_json::Value) -> Self {
        let now = Utc::now();
        Self {
            id: EpisodeId::new(),
            name: name.into(),
            source_type: EpisodeType::Json,
            source_description: String::new(),
            content: content.to_string(),
            group_id: String::from("default"),
            entity_edge_ids: Vec::new(),
            valid_at: now,
            created_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Builder: set group ID.
    pub fn with_group_id(mut self, group_id: impl Into<String>) -> Self {
        self.group_id = group_id.into();
        self
    }

    /// Builder: set source description.
    pub fn with_source_description(mut self, desc: impl Into<String>) -> Self {
        self.source_description = desc.into();
        self
    }

    /// Builder: set valid_at timestamp.
    pub fn with_valid_at(mut self, valid_at: DateTime<Utc>) -> Self {
        self.valid_at = valid_at;
        self
    }
}

impl PartialEq for Episode {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Episode {}

impl std::hash::Hash for Episode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_episode() {
        let ep = Episode::text("test-ep", "Alice moved to SF");
        assert_eq!(ep.name, "test-ep");
        assert_eq!(ep.source_type, EpisodeType::Text);
        assert_eq!(ep.content, "Alice moved to SF");
    }

    #[test]
    fn test_json_episode() {
        let ep = Episode::json("json-ep", serde_json::json!({"name": "Bob"}));
        assert_eq!(ep.source_type, EpisodeType::Json);
        assert!(ep.content.contains("Bob"));
    }
}
