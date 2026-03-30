// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Temporal Manager — point-in-time graph snapshots and temporal queries.
//!
//! This module implements the core temporal functionality:
//! - `snapshot_at(timestamp)`: Returns entities and relationships valid at a given time
//! - `entities_valid_at(timestamp)`: Filter entities by creation time
//! - `relationships_valid_at(timestamp)`: Filter relationships by bi-temporal validity
//! - `compact_timeline()`: Generate timeline events for G6 Timebar integration
//!
//! The bi-temporal model comes from Graphiti:
//! - **Valid time**: When the fact was true in reality (valid_from, valid_until)
//! - **Transaction time**: When we recorded/invalidated it (created_at, expired_at)
//!
//! This enables two types of temporal queries:
//! 1. "What was true on date X?" (valid time)
//! 2. "What did we know on date X?" (transaction time)

use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};

use crate::drivers::traits::GraphDriver;
use crate::graph::{Entity, Relationship};

/// A point-in-time snapshot of the knowledge graph.
#[derive(Debug, Clone)]
pub struct TemporalSnapshot {
    /// The timestamp this snapshot represents.
    pub timestamp: DateTime<Utc>,
    /// Entities that existed at this timestamp.
    pub entities: Vec<Entity>,
    /// Relationships that were valid at this timestamp.
    pub relationships: Vec<Relationship>,
    /// Total entity count at this time.
    pub entity_count: usize,
    /// Total valid relationship count at this time.
    pub relationship_count: usize,
}

/// A timeline event for visualization (maps to G6 Timebar data).
#[derive(Debug, Clone, serde::Serialize)]
pub struct TimelineEvent {
    /// When this event occurred.
    pub timestamp: DateTime<Utc>,
    /// Type of event.
    pub event_type: TimelineEventType,
    /// Human-readable description.
    pub description: String,
    /// Number of entities affected.
    pub entity_count: usize,
    /// Number of relationships affected.
    pub relationship_count: usize,
}

/// Types of timeline events.
#[derive(Debug, Clone, serde::Serialize)]
pub enum TimelineEventType {
    /// New entities/relationships were added.
    Ingestion,
    /// Existing facts were invalidated/updated.
    Update,
    /// Facts expired due to contradiction.
    Contradiction,
    /// Graph snapshot point.
    Snapshot,
}

/// The Temporal Manager orchestrates all time-related operations.
pub struct TemporalManager {
    driver: Arc<dyn GraphDriver>,
}

impl TemporalManager {
    /// Create a new temporal manager.
    pub fn new(driver: Arc<dyn GraphDriver>) -> Self {
        Self { driver }
    }

    /// Get a point-in-time snapshot of the knowledge graph.
    ///
    /// Returns all entities and relationships that were valid at the given timestamp.
    /// This uses the bi-temporal model:
    /// - Entity must have been created before `timestamp`
    /// - Relationship must satisfy `valid_from <= timestamp < valid_until`
    /// - Relationship must not have been expired before `timestamp`
    pub async fn snapshot_at(
        &self,
        timestamp: DateTime<Utc>,
        group_id: Option<&str>,
    ) -> Result<TemporalSnapshot, TemporalError> {
        // Get snapshot from driver (uses driver's built-in temporal filtering)
        let subgraph = self
            .driver
            .snapshot_at(&timestamp, group_id)
            .await
            .map_err(|e| TemporalError::DriverError(e.to_string()))?;

        let entity_count = subgraph.entities.len();
        let relationship_count = subgraph.relationships.len();

        Ok(TemporalSnapshot {
            timestamp,
            entities: subgraph.entities,
            relationships: subgraph.relationships,
            entity_count,
            relationship_count,
        })
    }

    /// Parse a date string into a DateTime<Utc>.
    ///
    /// Supports multiple formats:
    /// - "2025-01-15" (ISO date)
    /// - "2025-01-15T10:30:00Z" (ISO datetime)
    /// - "January 15, 2025" (natural language — basic)
    /// - "2025" (year only → Jan 1 of that year)
    pub fn parse_date(date_str: &str) -> Result<DateTime<Utc>, TemporalError> {
        let trimmed = date_str.trim();

        // Try ISO datetime first
        if let Ok(dt) = trimmed.parse::<DateTime<Utc>>() {
            return Ok(dt);
        }

        // Try ISO date (YYYY-MM-DD)
        if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
            return Ok(date
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc());
        }

        // Try year only
        if trimmed.len() == 4 {
            if let Ok(year) = trimmed.parse::<i32>() {
                if let Some(date) = NaiveDate::from_ymd_opt(year, 1, 1) {
                    return Ok(date
                        .and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_utc());
                }
            }
        }

        // Try "Month DD, YYYY" format
        let month_formats = [
            "%B %d, %Y",   // January 15, 2025
            "%b %d, %Y",   // Jan 15, 2025
            "%B %Y",       // January 2025
            "%b %Y",       // Jan 2025
            "%Y/%m/%d",    // 2025/01/15
            "%m/%d/%Y",    // 01/15/2025
            "%d-%m-%Y",    // 15-01-2025
        ];

        for fmt in &month_formats {
            if let Ok(date) = NaiveDate::parse_from_str(trimmed, fmt) {
                return Ok(date
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc());
            }
        }

        Err(TemporalError::InvalidDate(format!(
            "Cannot parse date: '{}'. Expected formats: YYYY-MM-DD, YYYY-MM-DDTHH:MM:SSZ, YYYY",
            date_str
        )))
    }

    /// Get the timeline of all events in the graph.
    ///
    /// This generates a sequence of events suitable for a timeline visualization
    /// (G6 Timebar). Events are sorted chronologically.
    pub async fn build_timeline(
        &self,
        group_id: Option<&str>,
    ) -> Result<Vec<TimelineEvent>, TemporalError> {
        let mut events = Vec::new();

        // Get all entities and extract creation timestamps
        let entities = self
            .driver
            .list_entities(group_id, 10000)
            .await
            .map_err(|e| TemporalError::DriverError(e.to_string()))?;

        // Group entities by creation date
        let mut creation_dates = std::collections::BTreeMap::<String, Vec<&Entity>>::new();
        for entity in &entities {
            let date_key = entity.created_at.format("%Y-%m-%d").to_string();
            creation_dates.entry(date_key).or_default().push(entity);
        }

        for (date_str, date_entities) in &creation_dates {
            if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                events.push(TimelineEvent {
                    timestamp: date
                        .and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_utc(),
                    event_type: TimelineEventType::Ingestion,
                    description: format!(
                        "Added {} entities: {}",
                        date_entities.len(),
                        date_entities
                            .iter()
                            .take(3)
                            .map(|e| e.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    entity_count: date_entities.len(),
                    relationship_count: 0,
                });
            }
        }

        // Sort by timestamp
        events.sort_by_key(|e| e.timestamp);

        Ok(events)
    }

    /// Get what changed between two timestamps.
    ///
    /// Returns entities and relationships that were created, modified, or
    /// invalidated in the given time range.
    pub async fn what_changed(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        group_id: Option<&str>,
    ) -> Result<TemporalDiff, TemporalError> {
        let entities = self
            .driver
            .list_entities(group_id, 10000)
            .await
            .map_err(|e| TemporalError::DriverError(e.to_string()))?;

        let mut added_entities = Vec::new();
        let mut modified_entities = Vec::new();

        for entity in entities {
            if entity.created_at >= from && entity.created_at <= to {
                added_entities.push(entity);
            } else if entity.updated_at >= from && entity.updated_at <= to {
                modified_entities.push(entity);
            }
        }

        Ok(TemporalDiff {
            from,
            to,
            added_entities,
            modified_entities,
            invalidated_relationships: Vec::new(), // TODO: query invalidated rels
        })
    }
}

/// Difference between two temporal snapshots.
#[derive(Debug, Clone)]
pub struct TemporalDiff {
    /// Start of the diff window.
    pub from: DateTime<Utc>,
    /// End of the diff window.
    pub to: DateTime<Utc>,
    /// Entities created in this window.
    pub added_entities: Vec<Entity>,
    /// Entities modified in this window.
    pub modified_entities: Vec<Entity>,
    /// Relationships invalidated in this window.
    pub invalidated_relationships: Vec<Relationship>,
}

/// Errors from temporal operations.
#[derive(Debug, thiserror::Error)]
pub enum TemporalError {
    #[error("Invalid date: {0}")]
    InvalidDate(String),

    #[error("Driver error: {0}")]
    DriverError(String),

    #[error("Temporal range error: from ({from}) must be before to ({to})")]
    InvalidRange {
        from: String,
        to: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iso_date() {
        let dt = TemporalManager::parse_date("2025-01-15").unwrap();
        assert_eq!(dt.date_naive().to_string(), "2025-01-15");
    }

    #[test]
    fn test_parse_iso_datetime() {
        let dt = TemporalManager::parse_date("2025-01-15T10:30:00Z").unwrap();
        assert_eq!(dt.date_naive().to_string(), "2025-01-15");
    }

    #[test]
    fn test_parse_year_only() {
        let dt = TemporalManager::parse_date("2025").unwrap();
        assert_eq!(dt.date_naive().to_string(), "2025-01-01");
    }

    #[test]
    fn test_parse_invalid_date() {
        assert!(TemporalManager::parse_date("not a date").is_err());
    }

    #[test]
    fn test_parse_slash_date() {
        let dt = TemporalManager::parse_date("2025/01/15").unwrap();
        assert_eq!(dt.date_naive().to_string(), "2025-01-15");
    }

    #[tokio::test]
    async fn test_snapshot_at() {
        use crate::drivers::memory::MemoryDriver;

        let driver = Arc::new(MemoryDriver::new());
        let manager = TemporalManager::new(driver.clone());

        // Store an entity
        let entity = Entity::new("Alice", "Person");
        driver.store_entity(&entity).await.unwrap();

        // Snapshot at now should include Alice
        let snapshot = manager.snapshot_at(Utc::now(), None).await.unwrap();
        assert_eq!(snapshot.entity_count, 1);
        assert_eq!(snapshot.entities[0].name, "Alice");
    }

    #[tokio::test]
    async fn test_what_changed() {
        use crate::drivers::memory::MemoryDriver;

        let driver = Arc::new(MemoryDriver::new());
        let manager = TemporalManager::new(driver.clone());

        let before = Utc::now();

        // Store entity
        let entity = Entity::new("Alice", "Person");
        driver.store_entity(&entity).await.unwrap();

        let after = Utc::now();

        let diff = manager.what_changed(before, after, None).await.unwrap();
        assert_eq!(diff.added_entities.len(), 1);
        assert_eq!(diff.added_entities[0].name, "Alice");
    }

    #[tokio::test]
    async fn test_build_timeline() {
        use crate::drivers::memory::MemoryDriver;

        let driver = Arc::new(MemoryDriver::new());
        let manager = TemporalManager::new(driver.clone());

        driver.store_entity(&Entity::new("Alice", "Person")).await.unwrap();
        driver.store_entity(&Entity::new("Bob", "Person")).await.unwrap();

        let timeline = manager.build_timeline(None).await.unwrap();
        assert!(!timeline.is_empty());
    }
}
