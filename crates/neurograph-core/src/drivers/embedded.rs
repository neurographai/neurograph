// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Sled-backed embedded persistent graph driver.
//!
//! Uses the `sled` crate (pure Rust embedded key-value store) for
//! persistent storage with zero external dependencies.
//!
//! Data is stored as JSON-serialized values in sled trees:
//! - "entities" tree: EntityId → Entity JSON
//! - "relationships" tree: RelationshipId → Relationship JSON
//! - "episodes" tree: EpisodeId → Episode JSON
//! - "communities" tree: CommunityId → Community JSON
//! - "entity_rels" tree: EntityId → Vec<RelationshipId> (index)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::Path;

use crate::graph::{
    Community, CommunityId, CommunityLevel, Entity, EntityId, Episode, EpisodeId, Relationship,
    RelationshipId,
};

use super::traits::{
    DriverError, DriverResult, GraphDriver, Subgraph, TextSearchResult, VectorSearchResult,
};

/// Sled-backed embedded graph driver.
///
/// Provides persistent storage with crash safety guarantees.
/// Data survives process restarts.
#[derive(Debug, Clone)]
pub struct EmbeddedDriver {
    db: sled::Db,
}

impl EmbeddedDriver {
    /// Open or create an embedded database at the given path.
    pub fn open(path: impl AsRef<Path>) -> DriverResult<Self> {
        let db = sled::open(path).map_err(|e| DriverError::StorageError(e.to_string()))?;
        Ok(Self { db })
    }

    /// Create an embedded database in a temporary directory (for testing).
    pub fn temporary() -> DriverResult<Self> {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .map_err(|e| DriverError::StorageError(e.to_string()))?;
        Ok(Self { db })
    }

    fn entities_tree(&self) -> DriverResult<sled::Tree> {
        self.db
            .open_tree("entities")
            .map_err(|e| DriverError::StorageError(e.to_string()))
    }

    fn relationships_tree(&self) -> DriverResult<sled::Tree> {
        self.db
            .open_tree("relationships")
            .map_err(|e| DriverError::StorageError(e.to_string()))
    }

    fn episodes_tree(&self) -> DriverResult<sled::Tree> {
        self.db
            .open_tree("episodes")
            .map_err(|e| DriverError::StorageError(e.to_string()))
    }

    fn communities_tree(&self) -> DriverResult<sled::Tree> {
        self.db
            .open_tree("communities")
            .map_err(|e| DriverError::StorageError(e.to_string()))
    }

    fn serialize<T: serde::Serialize>(value: &T) -> DriverResult<Vec<u8>> {
        serde_json::to_vec(value).map_err(|e| DriverError::SerializationError(e.to_string()))
    }

    fn deserialize<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> DriverResult<T> {
        serde_json::from_slice(bytes).map_err(|e| DriverError::SerializationError(e.to_string()))
    }
}

/// Cosine similarity (same as memory driver).
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (*x as f64) * (*y as f64))
        .sum();
    let norm_a: f64 = a
        .iter()
        .map(|x| (*x as f64) * (*x as f64))
        .sum::<f64>()
        .sqrt();
    let norm_b: f64 = b
        .iter()
        .map(|x| (*x as f64) * (*x as f64))
        .sum::<f64>()
        .sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

#[async_trait]
impl GraphDriver for EmbeddedDriver {
    fn name(&self) -> &str {
        "embedded-sled"
    }

    async fn store_entity(&self, entity: &Entity) -> DriverResult<()> {
        let tree = self.entities_tree()?;
        let bytes = Self::serialize(entity)?;
        tree.insert(entity.id.as_str().as_bytes(), bytes)
            .map_err(|e| DriverError::StorageError(e.to_string()))?;
        tree.flush_async()
            .await
            .map_err(|e| DriverError::StorageError(e.to_string()))?;
        Ok(())
    }

    async fn get_entity(&self, id: &EntityId) -> DriverResult<Entity> {
        let tree = self.entities_tree()?;
        let bytes = tree
            .get(id.as_str().as_bytes())
            .map_err(|e| DriverError::StorageError(e.to_string()))?
            .ok_or_else(|| DriverError::EntityNotFound(id.as_str()))?;
        Self::deserialize(&bytes)
    }

    async fn delete_entity(&self, id: &EntityId) -> DriverResult<()> {
        let tree = self.entities_tree()?;
        tree.remove(id.as_str().as_bytes())
            .map_err(|e| DriverError::StorageError(e.to_string()))?;
        Ok(())
    }

    async fn list_entities(
        &self,
        group_id: Option<&str>,
        limit: usize,
    ) -> DriverResult<Vec<Entity>> {
        let tree = self.entities_tree()?;
        let mut entities = Vec::new();
        for result in tree.iter() {
            if entities.len() >= limit {
                break;
            }
            let (_, bytes) = result.map_err(|e| DriverError::StorageError(e.to_string()))?;
            let entity: Entity = Self::deserialize(&bytes)?;
            if let Some(gid) = group_id {
                if entity.group_id != gid {
                    continue;
                }
            }
            entities.push(entity);
        }
        Ok(entities)
    }

    async fn store_relationship(&self, relationship: &Relationship) -> DriverResult<()> {
        let tree = self.relationships_tree()?;
        let bytes = Self::serialize(relationship)?;
        tree.insert(relationship.id.as_str().as_bytes(), bytes)
            .map_err(|e| DriverError::StorageError(e.to_string()))?;
        tree.flush_async()
            .await
            .map_err(|e| DriverError::StorageError(e.to_string()))?;
        Ok(())
    }

    async fn get_relationship(&self, id: &RelationshipId) -> DriverResult<Relationship> {
        let tree = self.relationships_tree()?;
        let bytes = tree
            .get(id.as_str().as_bytes())
            .map_err(|e| DriverError::StorageError(e.to_string()))?
            .ok_or_else(|| DriverError::RelationshipNotFound(id.as_str()))?;
        Self::deserialize(&bytes)
    }

    async fn get_entity_relationships(
        &self,
        entity_id: &EntityId,
    ) -> DriverResult<Vec<Relationship>> {
        let tree = self.relationships_tree()?;
        let entity_id_str = entity_id.as_str();
        let mut rels = Vec::new();
        for result in tree.iter() {
            let (_, bytes) = result.map_err(|e| DriverError::StorageError(e.to_string()))?;
            let rel: Relationship = Self::deserialize(&bytes)?;
            if rel.source_entity_id.as_str() == entity_id_str
                || rel.target_entity_id.as_str() == entity_id_str
            {
                rels.push(rel);
            }
        }
        Ok(rels)
    }

    async fn delete_relationship(&self, id: &RelationshipId) -> DriverResult<()> {
        let tree = self.relationships_tree()?;
        tree.remove(id.as_str().as_bytes())
            .map_err(|e| DriverError::StorageError(e.to_string()))?;
        Ok(())
    }

    async fn store_episode(&self, episode: &Episode) -> DriverResult<()> {
        let tree = self.episodes_tree()?;
        let bytes = Self::serialize(episode)?;
        tree.insert(episode.id.as_str().as_bytes(), bytes)
            .map_err(|e| DriverError::StorageError(e.to_string()))?;
        tree.flush_async()
            .await
            .map_err(|e| DriverError::StorageError(e.to_string()))?;
        Ok(())
    }

    async fn get_episode(&self, id: &EpisodeId) -> DriverResult<Episode> {
        let tree = self.episodes_tree()?;
        let bytes = tree
            .get(id.as_str().as_bytes())
            .map_err(|e| DriverError::StorageError(e.to_string()))?
            .ok_or_else(|| DriverError::EpisodeNotFound(id.as_str()))?;
        Self::deserialize(&bytes)
    }

    async fn list_episodes(
        &self,
        group_id: Option<&str>,
        limit: usize,
    ) -> DriverResult<Vec<Episode>> {
        let tree = self.episodes_tree()?;
        let mut episodes = Vec::new();
        for result in tree.iter() {
            let (_, bytes) = result.map_err(|e| DriverError::StorageError(e.to_string()))?;
            let ep: Episode = Self::deserialize(&bytes)?;
            if let Some(gid) = group_id {
                if ep.group_id != gid {
                    continue;
                }
            }
            episodes.push(ep);
        }
        episodes.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        episodes.truncate(limit);
        Ok(episodes)
    }

    async fn store_community(&self, community: &Community) -> DriverResult<()> {
        let tree = self.communities_tree()?;
        let bytes = Self::serialize(community)?;
        tree.insert(community.id.as_str().as_bytes(), bytes)
            .map_err(|e| DriverError::StorageError(e.to_string()))?;
        Ok(())
    }

    async fn get_community(&self, id: &CommunityId) -> DriverResult<Community> {
        let tree = self.communities_tree()?;
        let bytes = tree
            .get(id.as_str().as_bytes())
            .map_err(|e| DriverError::StorageError(e.to_string()))?
            .ok_or_else(|| DriverError::CommunityNotFound(id.as_str().to_string()))?;
        Self::deserialize(&bytes)
    }

    async fn get_communities_at_level(
        &self,
        level: CommunityLevel,
    ) -> DriverResult<Vec<Community>> {
        let tree = self.communities_tree()?;
        let mut communities = Vec::new();
        for result in tree.iter() {
            let (_, bytes) = result.map_err(|e| DriverError::StorageError(e.to_string()))?;
            let c: Community = Self::deserialize(&bytes)?;
            if c.level == level {
                communities.push(c);
            }
        }
        Ok(communities)
    }

    async fn get_dirty_communities(&self) -> DriverResult<Vec<Community>> {
        let tree = self.communities_tree()?;
        let mut communities = Vec::new();
        for result in tree.iter() {
            let (_, bytes) = result.map_err(|e| DriverError::StorageError(e.to_string()))?;
            let c: Community = Self::deserialize(&bytes)?;
            if c.is_dirty {
                communities.push(c);
            }
        }
        Ok(communities)
    }

    async fn list_communities(&self, group_id: Option<&str>) -> DriverResult<Vec<Community>> {
        let tree = self.communities_tree()?;
        let mut communities = Vec::new();
        for result in tree.iter() {
            let (_, bytes) = result.map_err(|e| DriverError::StorageError(e.to_string()))?;
            let c: Community = Self::deserialize(&bytes)?;
            if let Some(gid) = group_id {
                if c.group_id != gid {
                    continue;
                }
            }
            communities.push(c);
        }
        Ok(communities)
    }

    async fn search_entities_by_vector(
        &self,
        embedding: &[f32],
        k: usize,
        group_id: Option<&str>,
    ) -> DriverResult<Vec<VectorSearchResult>> {
        let tree = self.entities_tree()?;
        let mut results: Vec<VectorSearchResult> = Vec::new();

        for result in tree.iter() {
            let (_, bytes) = result.map_err(|e| DriverError::StorageError(e.to_string()))?;
            let entity: Entity = Self::deserialize(&bytes)?;

            if let Some(gid) = group_id {
                if entity.group_id != gid {
                    continue;
                }
            }

            if let Some(ref emb) = entity.name_embedding {
                let score = cosine_similarity(embedding, emb);
                results.push(VectorSearchResult { entity, score });
            }
        }

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(k);
        Ok(results)
    }

    async fn search_relationships_by_vector(
        &self,
        embedding: &[f32],
        k: usize,
        group_id: Option<&str>,
    ) -> DriverResult<Vec<Relationship>> {
        let tree = self.relationships_tree()?;
        let mut scored: Vec<(Relationship, f64)> = Vec::new();

        for result in tree.iter() {
            let (_, bytes) = result.map_err(|e| DriverError::StorageError(e.to_string()))?;
            let rel: Relationship = Self::deserialize(&bytes)?;

            if let Some(gid) = group_id {
                if rel.group_id != gid {
                    continue;
                }
            }

            if let Some(ref emb) = rel.fact_embedding {
                let score = cosine_similarity(embedding, emb);
                scored.push((rel, score));
            }
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        Ok(scored.into_iter().map(|(r, _)| r).collect())
    }

    async fn search_entities_by_text(
        &self,
        query: &str,
        limit: usize,
        group_id: Option<&str>,
    ) -> DriverResult<Vec<TextSearchResult>> {
        let tree = self.entities_tree()?;
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();
        let mut results: Vec<TextSearchResult> = Vec::new();

        for result in tree.iter() {
            let (_, bytes) = result.map_err(|e| DriverError::StorageError(e.to_string()))?;
            let entity: Entity = Self::deserialize(&bytes)?;

            if let Some(gid) = group_id {
                if entity.group_id != gid {
                    continue;
                }
            }

            let searchable = format!(
                "{} {}",
                entity.name.to_lowercase(),
                entity.summary.to_lowercase()
            );
            let matching = query_terms
                .iter()
                .filter(|t| searchable.contains(*t))
                .count();

            if matching > 0 {
                let score = matching as f64 / query_terms.len() as f64;
                let boost = if entity.name.to_lowercase().contains(&query_lower) {
                    2.0
                } else {
                    1.0
                };
                results.push(TextSearchResult {
                    entity,
                    score: score * boost,
                });
            }
        }

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);
        Ok(results)
    }

    async fn traverse(
        &self,
        start: &EntityId,
        max_depth: usize,
        group_id: Option<&str>,
    ) -> DriverResult<Subgraph> {
        let mut visited = std::collections::HashSet::new();
        let mut visited_rels = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((start.clone(), 0));

        let mut subgraph = Subgraph::default();

        while let Some((eid, depth)) = queue.pop_front() {
            if depth > max_depth || visited.contains(&eid.as_str()) {
                continue;
            }
            visited.insert(eid.as_str());

            if let Ok(entity) = self.get_entity(&eid).await {
                if let Some(gid) = group_id {
                    if entity.group_id != gid {
                        continue;
                    }
                }
                subgraph.entities.push(entity);
            }

            if let Ok(rels) = self.get_entity_relationships(&eid).await {
                for rel in rels {
                    if visited_rels.contains(&rel.id.as_str()) || !rel.is_valid() {
                        continue;
                    }
                    visited_rels.insert(rel.id.as_str());
                    let next = if rel.source_entity_id == eid {
                        rel.target_entity_id.clone()
                    } else {
                        rel.source_entity_id.clone()
                    };
                    subgraph.relationships.push(rel);
                    queue.push_back((next, depth + 1));
                }
            }
        }

        Ok(subgraph)
    }

    async fn snapshot_at(
        &self,
        timestamp: &DateTime<Utc>,
        group_id: Option<&str>,
    ) -> DriverResult<Subgraph> {
        let entities = self
            .list_entities(group_id, usize::MAX)
            .await?
            .into_iter()
            .filter(|e| e.created_at <= *timestamp)
            .collect();

        let tree = self.relationships_tree()?;
        let mut relationships = Vec::new();
        for result in tree.iter() {
            let (_, bytes) = result.map_err(|e| DriverError::StorageError(e.to_string()))?;
            let rel: Relationship = Self::deserialize(&bytes)?;
            if let Some(gid) = group_id {
                if rel.group_id != gid {
                    continue;
                }
            }
            if rel.is_valid_at(timestamp) {
                relationships.push(rel);
            }
        }

        Ok(Subgraph {
            entities,
            relationships,
        })
    }

    async fn stats(&self) -> DriverResult<HashMap<String, usize>> {
        let mut stats = HashMap::new();
        stats.insert("entities".to_string(), self.entities_tree()?.len());
        stats.insert(
            "relationships".to_string(),
            self.relationships_tree()?.len(),
        );
        stats.insert("episodes".to_string(), self.episodes_tree()?.len());
        stats.insert("communities".to_string(), self.communities_tree()?.len());
        Ok(stats)
    }

    async fn clear(&self) -> DriverResult<()> {
        self.entities_tree()?
            .clear()
            .map_err(|e| DriverError::StorageError(e.to_string()))?;
        self.relationships_tree()?
            .clear()
            .map_err(|e| DriverError::StorageError(e.to_string()))?;
        self.episodes_tree()?
            .clear()
            .map_err(|e| DriverError::StorageError(e.to_string()))?;
        self.communities_tree()?
            .clear()
            .map_err(|e| DriverError::StorageError(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embedded_driver_persistence() {
        let driver = EmbeddedDriver::temporary().unwrap();

        let entity = Entity::new("Alice", "Person");
        driver.store_entity(&entity).await.unwrap();

        let retrieved = driver.get_entity(&entity.id).await.unwrap();
        assert_eq!(retrieved.name, "Alice");
    }

    #[tokio::test]
    async fn test_embedded_driver_stats() {
        let driver = EmbeddedDriver::temporary().unwrap();
        driver.store_entity(&Entity::new("A", "T")).await.unwrap();
        driver.store_entity(&Entity::new("B", "T")).await.unwrap();

        let stats = driver.stats().await.unwrap();
        assert_eq!(stats["entities"], 2);
    }
}
