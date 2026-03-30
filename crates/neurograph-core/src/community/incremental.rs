// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Incremental community updates.
//!
//! When new entities are ingested or relationships change, we don't
//! recompute all communities from scratch. Instead, we:
//! 1. Identify affected communities (those containing modified entities)
//! 2. Mark them as "dirty" for re-summarization
//! 3. Optionally re-run Louvain on the local neighborhood
//!
//! This is a key differentiator from GraphRAG which requires full reindexing.

use std::sync::Arc;

use crate::drivers::traits::GraphDriver;
use crate::graph::entity::EntityId;
use crate::graph::Community;

/// Result of an incremental community update.
#[derive(Debug, Clone)]
pub struct IncrementalUpdateResult {
    /// Communities that were marked dirty.
    pub dirty_communities: usize,
    /// New communities created (for entities not in any community).
    pub new_communities: usize,
    /// Entities processed.
    pub entities_processed: usize,
}

/// Manages incremental community updates.
pub struct IncrementalCommunityUpdater {
    driver: Arc<dyn GraphDriver>,
}

impl IncrementalCommunityUpdater {
    /// Create a new updater.
    pub fn new(driver: Arc<dyn GraphDriver>) -> Self {
        Self { driver }
    }

    /// Update communities after new entities are ingested.
    ///
    /// For each entity:
    /// 1. Check if it belongs to an existing community
    /// 2. If yes, mark that community as dirty (needs re-summarization)
    /// 3. If no, assign it to the most connected community or create a singleton
    pub async fn update_after_ingestion(
        &self,
        entity_ids: &[EntityId],
        group_id: Option<&str>,
    ) -> Result<IncrementalUpdateResult, IncrementalError> {
        let mut dirty_count = 0;
        let mut new_count = 0;

        let communities = self
            .driver
            .list_communities(group_id)
            .await
            .map_err(|e| IncrementalError::DriverError(e.to_string()))?;

        // Build member → community index
        let mut entity_to_community: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        for community in &communities {
            for member_id in community.members() {
                entity_to_community.insert(member_id.as_str(), community.id.as_str().to_string());
            }
        }

        let mut dirty_community_ids = std::collections::HashSet::new();

        for entity_id in entity_ids {
            let entity_id_str = entity_id.as_str();
            if let Some(comm_id) = entity_to_community.get(&entity_id_str) {
                // Entity is already in a community — mark it dirty
                dirty_community_ids.insert(comm_id.clone());
            } else {
                // Entity is not in any community — try to assign it
                let assigned = self
                    .assign_to_best_community(entity_id, &communities)
                    .await?;

                if let Some(comm_id) = assigned {
                    dirty_community_ids.insert(comm_id);
                } else {
                    // Create a new singleton community
                    let entity = self
                        .driver
                        .get_entity(entity_id)
                        .await
                        .map_err(|e| IncrementalError::DriverError(e.to_string()))?;

                    let mut new_comm = Community::new(
                        format!("community_{}", entity.name.to_lowercase().replace(' ', "_")),
                        0,
                    );
                    new_comm.add_member(entity_id.clone());
                    new_comm.is_dirty = true;

                    self.driver
                        .store_community(&new_comm)
                        .await
                        .map_err(|e| IncrementalError::DriverError(e.to_string()))?;

                    new_count += 1;
                }
            }
        }

        // Mark dirty communities
        for comm_id in &dirty_community_ids {
            // Find and update the community
            for community in &communities {
                if community.id.as_str() == comm_id.as_str() {
                    let mut updated = community.clone();
                    updated.is_dirty = true;
                    self.driver
                        .store_community(&updated)
                        .await
                        .map_err(|e| IncrementalError::DriverError(e.to_string()))?;
                    dirty_count += 1;
                    break;
                }
            }
        }

        Ok(IncrementalUpdateResult {
            dirty_communities: dirty_count,
            new_communities: new_count,
            entities_processed: entity_ids.len(),
        })
    }

    /// Try to assign an entity to the best existing community based on connectivity.
    ///
    /// Looks at the entity's relationships. If most of its neighbors are in community X,
    /// assign this entity to community X.
    async fn assign_to_best_community(
        &self,
        entity_id: &EntityId,
        communities: &[Community],
    ) -> Result<Option<String>, IncrementalError> {
        // Get relationships for this entity
        let rels = self
            .driver
            .get_entity_relationships(entity_id)
            .await
            .unwrap_or_default();

        if rels.is_empty() {
            return Ok(None);
        }

        // Count how many neighbors are in each community
        let mut community_votes: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for rel in &rels {
            let neighbor_id = if rel.source_entity_id == *entity_id {
                &rel.target_entity_id
            } else {
                &rel.source_entity_id
            };

            for community in communities {
                if community.members().any(|m| m == neighbor_id) {
                    *community_votes.entry(community.id.as_str().to_string()).or_insert(0) += 1;
                }
            }
        }

        // Pick the community with the most votes
        if let Some((best_comm_id, _)) = community_votes.iter().max_by_key(|&(_, count)| count) {
            // Add entity to this community
            for community in communities {
                if community.id.as_str() == best_comm_id.as_str() {
                    let mut updated = community.clone();
                    updated.add_member(entity_id.clone());
                    updated.is_dirty = true;
                    self.driver
                        .store_community(&updated)
                        .await
                        .map_err(|e| IncrementalError::DriverError(e.to_string()))?;
                    return Ok(Some(best_comm_id.clone()));
                }
            }
        }

        Ok(None)
    }
}

/// Errors from incremental updates.
#[derive(Debug, thiserror::Error)]
pub enum IncrementalError {
    #[error("Driver error: {0}")]
    DriverError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::memory::MemoryDriver;
    use crate::graph::{Entity, Relationship};

    #[tokio::test]
    async fn test_incremental_new_entity() {
        let driver = Arc::new(MemoryDriver::new());

        // Store an entity that isn't in any community
        let alice = Entity::new("Alice", "Person");
        driver.store_entity(&alice).await.unwrap();

        let updater = IncrementalCommunityUpdater::new(driver.clone());
        let result = updater
            .update_after_ingestion(&[alice.id.clone()], None)
            .await
            .unwrap();

        // Should create a new singleton community
        assert_eq!(result.new_communities, 1);
        assert_eq!(result.entities_processed, 1);
    }

    #[tokio::test]
    async fn test_incremental_existing_community() {
        let driver = Arc::new(MemoryDriver::new());

        let alice = Entity::new("Alice", "Person");
        driver.store_entity(&alice).await.unwrap();

        // Create a community containing Alice
        let mut community = Community::new("test-comm", 0);
        community.add_member(alice.id.clone());
        driver.store_community(&community).await.unwrap();

        let updater = IncrementalCommunityUpdater::new(driver.clone());

        // Ingest a new entity that's not in any community but connected to Alice
        let bob = Entity::new("Bob", "Person");
        driver.store_entity(&bob).await.unwrap();

        let rel = Relationship::new(
            bob.id.clone(), alice.id.clone(),
            "KNOWS", "Bob knows Alice",
        );
        driver.store_relationship(&rel).await.unwrap();

        let result = updater
            .update_after_ingestion(&[bob.id.clone()], None)
            .await
            .unwrap();

        assert_eq!(result.entities_processed, 1);
    }
}
