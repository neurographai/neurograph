// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Temporal fact representation with validity windows.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::entity::EntityId;
use super::episode::EpisodeId;
use super::relationship::RelationshipId;

/// Temporal validity window for a fact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalValidity {
    /// When the fact became true in the real world.
    pub valid_from: Option<DateTime<Utc>>,
    /// When the fact stopped being true.
    pub valid_until: Option<DateTime<Utc>>,
    /// When we recorded this fact.
    pub recorded_at: DateTime<Utc>,
    /// When we invalidated this record (if ever).
    pub invalidated_at: Option<DateTime<Utc>>,
}

impl TemporalValidity {
    /// Create a new validity window starting now.
    pub fn from_now() -> Self {
        let now = Utc::now();
        Self {
            valid_from: Some(now),
            valid_until: None,
            recorded_at: now,
            invalidated_at: None,
        }
    }

    /// Create a validity window with a specific start time.
    pub fn from(valid_from: DateTime<Utc>) -> Self {
        Self {
            valid_from: Some(valid_from),
            valid_until: None,
            recorded_at: Utc::now(),
            invalidated_at: None,
        }
    }

    /// Check if the fact is currently valid.
    pub fn is_current(&self) -> bool {
        self.invalidated_at.is_none() && self.valid_until.is_none()
    }

    /// Check if the fact was valid at a specific point in time.
    pub fn is_valid_at(&self, timestamp: &DateTime<Utc>) -> bool {
        let after_start = self.valid_from.map(|vf| timestamp >= &vf).unwrap_or(true);

        let before_end = self.valid_until.map(|vu| timestamp < &vu).unwrap_or(true);

        after_start && before_end
    }
}

/// A temporal fact — a statement with a validity window and provenance.
///
/// Facts are the atomic unit of knowledge in NeuroGraph. They connect
/// entities via relationships and track when the fact was true.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalFact {
    /// The relationship that carries this fact.
    pub relationship_id: RelationshipId,

    /// Subject entity.
    pub subject_id: EntityId,

    /// Object entity.
    pub object_id: EntityId,

    /// The fact statement in natural language.
    pub statement: String,

    /// Temporal validity window.
    pub validity: TemporalValidity,

    /// Provenance — which episodes contributed this fact.
    pub episode_ids: Vec<EpisodeId>,

    /// If this fact was invalidated, what replaced it.
    pub superseded_by: Option<RelationshipId>,

    /// The reason for invalidation (if invalidated).
    pub invalidation_reason: Option<String>,
}

impl TemporalFact {
    /// Create a new temporal fact.
    pub fn new(
        relationship_id: RelationshipId,
        subject_id: EntityId,
        object_id: EntityId,
        statement: impl Into<String>,
    ) -> Self {
        Self {
            relationship_id,
            subject_id,
            object_id,
            statement: statement.into(),
            validity: TemporalValidity::from_now(),
            episode_ids: Vec::new(),
            superseded_by: None,
            invalidation_reason: None,
        }
    }

    /// Invalidate this fact and link to its replacement.
    pub fn invalidate(&mut self, reason: impl Into<String>, replaced_by: Option<RelationshipId>) {
        let now = Utc::now();
        self.validity.valid_until = Some(now);
        self.validity.invalidated_at = Some(now);
        self.invalidation_reason = Some(reason.into());
        self.superseded_by = replaced_by;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temporal_validity() {
        let validity = TemporalValidity::from_now();
        assert!(validity.is_current());
        assert!(validity.is_valid_at(&Utc::now()));
    }

    #[test]
    fn test_fact_invalidation() {
        let mut fact = TemporalFact::new(
            RelationshipId::new(),
            EntityId::new(),
            EntityId::new(),
            "Alice lives in NYC",
        );

        assert!(fact.validity.is_current());

        fact.invalidate("Alice moved to SF", None);
        assert!(!fact.validity.is_current());
        assert_eq!(
            fact.invalidation_reason.as_deref(),
            Some("Alice moved to SF")
        );
    }
}
