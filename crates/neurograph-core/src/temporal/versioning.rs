// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Fact version chains and entity history tracking.
//!
//! Tracks how facts evolve over time:
//! - Version chains: when a fact is superseded, link old → new
//! - Entity history: track all changes to an entity's attributes
//! - Diff API: compute what changed between two points in time
//!
//! Influenced by Graphiti's bi-temporal invalidation model where
//! facts are never deleted — they are marked with `invalid_at`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::graph::entity::EntityId;
use crate::graph::relationship::RelationshipId;

/// A single version in a fact's history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactVersion {
    /// The relationship ID this version relates to.
    pub relationship_id: RelationshipId,
    /// Version number (1 = original, 2 = first update, etc.)
    pub version: u32,
    /// The fact text at this version.
    pub fact: String,
    /// When this version became valid in reality.
    pub valid_from: DateTime<Utc>,
    /// When this version stopped being valid (None = still current).
    pub valid_until: Option<DateTime<Utc>>,
    /// When this version was recorded in the system.
    pub created_at: DateTime<Utc>,
    /// When this version was superseded in the system (None = current).
    pub superseded_at: Option<DateTime<Utc>>,
    /// The relationship ID that superseded this version (if any).
    pub superseded_by: Option<RelationshipId>,
    /// Confidence score at this version.
    pub confidence: f64,
}

/// A chain of fact versions showing how a relationship evolved.
#[derive(Debug, Clone)]
pub struct FactVersionChain {
    /// Source entity.
    pub source_entity_id: EntityId,
    /// Target entity.
    pub target_entity_id: EntityId,
    /// Relationship type.
    pub relationship_type: String,
    /// All versions, ordered chronologically.
    pub versions: Vec<FactVersion>,
}

impl FactVersionChain {
    /// Create a new version chain.
    pub fn new(
        source_entity_id: EntityId,
        target_entity_id: EntityId,
        relationship_type: impl Into<String>,
    ) -> Self {
        Self {
            source_entity_id,
            target_entity_id,
            relationship_type: relationship_type.into(),
            versions: Vec::new(),
        }
    }

    /// Add a version to the chain.
    pub fn add_version(&mut self, version: FactVersion) {
        self.versions.push(version);
        self.versions.sort_by_key(|v| v.created_at);
    }

    /// Get the current (latest valid) version.
    pub fn current(&self) -> Option<&FactVersion> {
        self.versions
            .iter()
            .rev()
            .find(|v| v.superseded_at.is_none())
    }

    /// Get the version that was valid at a specific time.
    pub fn version_at(&self, timestamp: &DateTime<Utc>) -> Option<&FactVersion> {
        self.versions.iter().rev().find(|v| {
            v.valid_from <= *timestamp
                && v.valid_until.is_none_or(|vu| *timestamp < vu)
                && v.superseded_at.is_none_or(|sa| *timestamp < sa)
        })
    }

    /// Total number of versions.
    pub fn version_count(&self) -> usize {
        self.versions.len()
    }
}

/// A record in an entity's modification history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityHistoryEntry {
    /// When this change occurred.
    pub timestamp: DateTime<Utc>,
    /// What type of change.
    pub change_type: EntityChangeType,
    /// Human-readable description of the change.
    pub description: String,
    /// Previous summary (if changed).
    pub old_summary: Option<String>,
    /// New summary (if changed).
    pub new_summary: Option<String>,
}

/// Types of entity changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityChangeType {
    /// Entity was created.
    Created,
    /// Entity summary was updated.
    SummaryUpdated,
    /// Entity was merged with another (deduplication).
    Merged,
    /// Entity type was changed.
    TypeChanged,
    /// Entity attributes were modified.
    AttributesUpdated,
    /// Entity was marked for decay/forgetting.
    DecayMarked,
}

/// Tracks entity history over time.
pub struct EntityHistory {
    /// The entity ID.
    pub entity_id: EntityId,
    /// All history entries, ordered chronologically.
    pub entries: Vec<EntityHistoryEntry>,
}

impl EntityHistory {
    /// Create a new history tracker for an entity.
    pub fn new(entity_id: EntityId) -> Self {
        Self {
            entity_id,
            entries: Vec::new(),
        }
    }

    /// Record a new change.
    pub fn record(&mut self, change_type: EntityChangeType, description: impl Into<String>) {
        self.entries.push(EntityHistoryEntry {
            timestamp: Utc::now(),
            change_type,
            description: description.into(),
            old_summary: None,
            new_summary: None,
        });
    }

    /// Record a summary change with before/after.
    pub fn record_summary_change(
        &mut self,
        old_summary: impl Into<String>,
        new_summary: impl Into<String>,
    ) {
        let old = old_summary.into();
        let new = new_summary.into();
        self.entries.push(EntityHistoryEntry {
            timestamp: Utc::now(),
            change_type: EntityChangeType::SummaryUpdated,
            description: format!("Summary updated from '{}' to '{}'", &old, &new),
            old_summary: Some(old),
            new_summary: Some(new),
        });
    }

    /// Get all changes in a time range.
    pub fn changes_between(
        &self,
        from: &DateTime<Utc>,
        to: &DateTime<Utc>,
    ) -> Vec<&EntityHistoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.timestamp >= *from && e.timestamp <= *to)
            .collect()
    }

    /// Get the most recent change.
    pub fn latest(&self) -> Option<&EntityHistoryEntry> {
        self.entries.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::entity::EntityId;
    use crate::graph::relationship::RelationshipId;

    #[test]
    fn test_fact_version_chain() {
        let src = EntityId::new();
        let tgt = EntityId::new();
        let mut chain = FactVersionChain::new(src, tgt, "LIVES_IN");

        let now = Utc::now();
        let past = now - chrono::Duration::days(365);

        // Version 1: Alice lives in NYC
        chain.add_version(FactVersion {
            relationship_id: RelationshipId::new(),
            version: 1,
            fact: "Alice lives in NYC".to_string(),
            valid_from: past,
            valid_until: Some(now),
            created_at: past,
            superseded_at: Some(now),
            superseded_by: None,
            confidence: 0.9,
        });

        // Version 2: Alice lives in SF
        chain.add_version(FactVersion {
            relationship_id: RelationshipId::new(),
            version: 2,
            fact: "Alice lives in San Francisco".to_string(),
            valid_from: now,
            valid_until: None,
            created_at: now,
            superseded_at: None,
            superseded_by: None,
            confidence: 0.95,
        });

        assert_eq!(chain.version_count(), 2);

        // Current should be SF
        let current = chain.current().unwrap();
        assert_eq!(current.fact, "Alice lives in San Francisco");

        // 6 months ago should be NYC
        let six_months_ago = now - chrono::Duration::days(180);
        let historical = chain.version_at(&six_months_ago).unwrap();
        assert_eq!(historical.fact, "Alice lives in NYC");
    }

    #[test]
    fn test_entity_history() {
        let entity_id = EntityId::new();
        let mut history = EntityHistory::new(entity_id);

        history.record(EntityChangeType::Created, "Entity created");
        history.record_summary_change("A person", "A researcher at Anthropic");

        assert_eq!(history.entries.len(), 2);

        let latest = history.latest().unwrap();
        assert!(matches!(latest.change_type, EntityChangeType::SummaryUpdated));
        assert_eq!(latest.new_summary.as_deref(), Some("A researcher at Anthropic"));
    }
}
