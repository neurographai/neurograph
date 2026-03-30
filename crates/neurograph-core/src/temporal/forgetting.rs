// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Intelligent decay and forgetting for knowledge graph maintenance.
//!
//! Implements strategies to manage graph growth:
//! - TTL-based expiration for time-sensitive facts
//! - Importance-based decay using PageRank + access frequency
//! - Configurable retention policies
//!
//! Influenced by Mem0's memory optimization (consolidation & pruning)
//! and enhanced with graph-aware importance scoring.

use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::drivers::traits::GraphDriver;
use crate::graph::Entity;

/// Configuration for the forgetting/decay system.
#[derive(Debug, Clone)]
pub struct ForgettingConfig {
    /// Enable automatic decay scoring.
    pub enabled: bool,
    /// Default TTL for facts without explicit expiration (None = never expire).
    pub default_ttl: Option<Duration>,
    /// Importance threshold below which entities are candidates for pruning.
    /// Range: 0.0 - 1.0 (default: 0.1)
    pub importance_threshold: f64,
    /// Minimum access count to prevent pruning (entities accessed more than
    /// this many times are always kept).
    pub min_access_count: u64,
    /// Decay rate per day for importance score (0.0 - 1.0).
    /// e.g., 0.01 means 1% decay per day since last access.
    pub daily_decay_rate: f64,
    /// Maximum number of entities to prune in a single pass.
    pub max_prune_batch: usize,
}

impl Default for ForgettingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_ttl: None,
            importance_threshold: 0.1,
            min_access_count: 5,
            daily_decay_rate: 0.01,
            max_prune_batch: 100,
        }
    }
}

/// Result of a decay/pruning pass.
#[derive(Debug, Clone)]
pub struct ForgettingResult {
    /// Entities whose importance scores were updated.
    pub scores_updated: usize,
    /// Entities identified as candidates for pruning.
    pub prune_candidates: usize,
    /// Entities actually pruned (if auto-prune is enabled).
    pub pruned: usize,
    /// Total entities evaluated.
    pub total_evaluated: usize,
}

/// The forgetting engine that manages knowledge decay.
pub struct ForgettingEngine {
    config: ForgettingConfig,
    driver: Arc<dyn GraphDriver>,
}

impl ForgettingEngine {
    /// Create a new forgetting engine.
    pub fn new(config: ForgettingConfig, driver: Arc<dyn GraphDriver>) -> Self {
        Self { config, driver }
    }

    /// Calculate the decayed importance score for an entity.
    ///
    /// The importance score decays based on:
    /// 1. Time since last access (exponential decay)
    /// 2. Total access count (log-scaled boost)
    /// 3. Number of relationships (connectivity boost)
    pub fn calculate_importance(
        &self,
        entity: &Entity,
        relationship_count: usize,
    ) -> f64 {
        if !self.config.enabled {
            return entity.importance_score;
        }

        let now = Utc::now();
        let days_since_update = (now - entity.updated_at).num_days().max(0) as f64;

        // Exponential decay based on time since last access
        let time_decay = (-self.config.daily_decay_rate * days_since_update).exp();

        // Access frequency boost (logarithmic scale to avoid runaway)
        let access_boost = (entity.access_count as f64 + 1.0).ln() / 10.0;

        // Connectivity boost (entities with more relationships are more important)
        let connectivity_boost = (relationship_count as f64 + 1.0).ln() / 10.0;

        // Combine: base importance × time_decay + bonuses
        let raw_score =
            entity.importance_score * time_decay + access_boost + connectivity_boost;

        // Clamp to [0.0, 1.0]
        raw_score.clamp(0.0, 1.0)
    }

    /// Run a decay pass: update importance scores and identify prune candidates.
    ///
    /// This does NOT delete anything unless `auto_prune` is true.
    pub async fn decay_pass(
        &self,
        auto_prune: bool,
        group_id: Option<&str>,
    ) -> Result<ForgettingResult, ForgettingError> {
        if !self.config.enabled {
            return Ok(ForgettingResult {
                scores_updated: 0,
                prune_candidates: 0,
                pruned: 0,
                total_evaluated: 0,
            });
        }

        let entities = self
            .driver
            .list_entities(group_id, 10000)
            .await
            .map_err(|e| ForgettingError::DriverError(e.to_string()))?;

        let total = entities.len();
        let mut scores_updated = 0;
        let mut prune_candidates = Vec::new();

        for entity in &entities {
            // Get relationship count for connectivity scoring
            let rel_count = self
                .driver
                .get_entity_relationships(&entity.id)
                .await
                .map(|r| r.len())
                .unwrap_or(0);

            let new_importance = self.calculate_importance(entity, rel_count);

            // Update if score changed significantly
            if (new_importance - entity.importance_score).abs() > 0.001 {
                let mut updated = entity.clone();
                updated.importance_score = new_importance;
                self.driver
                    .store_entity(&updated)
                    .await
                    .map_err(|e| ForgettingError::DriverError(e.to_string()))?;
                scores_updated += 1;
            }

            // Check if this entity is a prune candidate
            if new_importance < self.config.importance_threshold
                && entity.access_count < self.config.min_access_count
            {
                prune_candidates.push(entity.id.clone());
            }
        }

        let mut pruned = 0;
        if auto_prune {
            let to_prune = prune_candidates
                .iter()
                .take(self.config.max_prune_batch);

            for entity_id in to_prune {
                if self.driver.delete_entity(entity_id).await.is_ok() {
                    pruned += 1;
                }
            }
        }

        Ok(ForgettingResult {
            scores_updated,
            prune_candidates: prune_candidates.len(),
            pruned,
            total_evaluated: total,
        })
    }

    /// Check if an entity should be forgotten based on TTL.
    pub fn is_expired(&self, entity: &Entity) -> bool {
        if let Some(ttl) = self.config.default_ttl {
            let age = Utc::now() - entity.created_at;
            age > ttl
        } else {
            false
        }
    }

    /// Get entities that are candidates for pruning (low importance, low access).
    pub async fn get_prune_candidates(
        &self,
        group_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Entity>, ForgettingError> {
        let entities = self
            .driver
            .list_entities(group_id, 10000)
            .await
            .map_err(|e| ForgettingError::DriverError(e.to_string()))?;

        let mut candidates: Vec<Entity> = entities
            .into_iter()
            .filter(|e| {
                e.importance_score < self.config.importance_threshold
                    && e.access_count < self.config.min_access_count
            })
            .collect();

        // Sort by importance (lowest first)
        candidates.sort_by(|a, b| {
            a.importance_score
                .partial_cmp(&b.importance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        candidates.truncate(limit);
        Ok(candidates)
    }
}

/// Errors from forgetting operations.
#[derive(Debug, thiserror::Error)]
pub enum ForgettingError {
    #[error("Driver error: {0}")]
    DriverError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::memory::MemoryDriver;

    #[test]
    fn test_default_config() {
        let config = ForgettingConfig::default();
        assert!(!config.enabled);
        assert!(config.default_ttl.is_none());
        assert!(config.importance_threshold > 0.0);
    }

    #[test]
    fn test_importance_calculation() {
        let driver = Arc::new(MemoryDriver::new());
        let config = ForgettingConfig {
            enabled: true,
            daily_decay_rate: 0.01,
            ..Default::default()
        };
        let engine = ForgettingEngine::new(config, driver);

        // Fresh entity with high access count
        let mut entity = Entity::new("Alice", "Person");
        entity.access_count = 100;
        entity.importance_score = 0.8;

        let score = engine.calculate_importance(&entity, 5);
        assert!(score > 0.0, "Score should be positive");
        assert!(score <= 1.0, "Score should be <= 1.0");
    }

    #[test]
    fn test_importance_decays_over_time() {
        let driver = Arc::new(MemoryDriver::new());
        let config = ForgettingConfig {
            enabled: true,
            daily_decay_rate: 0.1, // Aggressive decay for testing
            ..Default::default()
        };
        let engine = ForgettingEngine::new(config, driver);

        // Entity last accessed 30 days ago
        let mut old_entity = Entity::new("OldEntity", "Test");
        old_entity.importance_score = 0.8;
        old_entity.access_count = 1;
        old_entity.updated_at = Utc::now() - Duration::days(30);

        // Fresh entity
        let mut fresh_entity = Entity::new("FreshEntity", "Test");
        fresh_entity.importance_score = 0.8;
        fresh_entity.access_count = 1;

        let old_score = engine.calculate_importance(&old_entity, 1);
        let fresh_score = engine.calculate_importance(&fresh_entity, 1);

        assert!(
            fresh_score > old_score,
            "Fresh entity ({:.3}) should have higher importance than old entity ({:.3})",
            fresh_score,
            old_score,
        );
    }

    #[test]
    fn test_ttl_expiration() {
        let driver = Arc::new(MemoryDriver::new());
        let config = ForgettingConfig {
            enabled: true,
            default_ttl: Some(Duration::days(7)),
            ..Default::default()
        };
        let engine = ForgettingEngine::new(config, driver);

        // Fresh entity — not expired
        let entity = Entity::new("Fresh", "Test");
        assert!(!engine.is_expired(&entity));

        // Old entity — expired
        let mut old = Entity::new("Old", "Test");
        old.created_at = Utc::now() - Duration::days(10);
        assert!(engine.is_expired(&old));
    }

    #[tokio::test]
    async fn test_decay_pass_disabled() {
        let driver = Arc::new(MemoryDriver::new());
        let config = ForgettingConfig::default(); // disabled by default
        let engine = ForgettingEngine::new(config, driver);

        let result = engine.decay_pass(false, None).await.unwrap();
        assert_eq!(result.total_evaluated, 0);
    }

    #[tokio::test]
    async fn test_prune_candidates() {
        let driver = Arc::new(MemoryDriver::new());

        // Store entities with varying importance
        let mut low = Entity::new("LowImportance", "Test");
        low.importance_score = 0.05;
        low.access_count = 1;
        driver.store_entity(&low).await.unwrap();

        let mut high = Entity::new("HighImportance", "Test");
        high.importance_score = 0.9;
        high.access_count = 100;
        driver.store_entity(&high).await.unwrap();

        let config = ForgettingConfig {
            enabled: true,
            importance_threshold: 0.1,
            min_access_count: 5,
            ..Default::default()
        };
        let engine = ForgettingEngine::new(config, driver);

        let candidates = engine.get_prune_candidates(None, 10).await.unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].name, "LowImportance");
    }
}
