// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! In-memory graph driver implementation using petgraph + HashMap.
//!
//! This is the zero-config default driver. No external database required.
//! Perfect for testing, prototyping, and small datasets (<100k entities).
//!
//! Uses:
//! - `DashMap` for concurrent entity/relationship/episode storage
//! - Brute-force cosine similarity for vector search (efficient up to ~50k vectors)
//! - Simple substring matching for text search

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;

use crate::graph::{
    Community, CommunityId, CommunityLevel, Entity, EntityId, Episode, EpisodeId, Relationship,
    RelationshipId,
};

use super::traits::{
    DriverError, DriverResult, GraphDriver, Subgraph, TextSearchResult, VectorSearchResult,
};

/// In-memory graph driver for zero-config operation.
///
/// Thread-safe via `DashMap` (lock-free concurrent hashmap).
/// All data lives in memory — lost on process exit.
#[derive(Debug, Clone)]
pub struct MemoryDriver {
    entities: Arc<DashMap<String, Entity>>,
    relationships: Arc<DashMap<String, Relationship>>,
    episodes: Arc<DashMap<String, Episode>>,
    communities: Arc<DashMap<String, Community>>,
}

impl MemoryDriver {
    /// Create a new in-memory driver.
    pub fn new() -> Self {
        Self {
            entities: Arc::new(DashMap::new()),
            relationships: Arc::new(DashMap::new()),
            episodes: Arc::new(DashMap::new()),
            communities: Arc::new(DashMap::new()),
        }
    }
}

impl Default for MemoryDriver {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute cosine similarity between two vectors.
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
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

#[async_trait]
impl GraphDriver for MemoryDriver {
    fn name(&self) -> &str {
        "memory"
    }

    // --- Entity Operations ---

    async fn store_entity(&self, entity: &Entity) -> DriverResult<()> {
        self.entities.insert(entity.id.as_str(), entity.clone());
        Ok(())
    }

    async fn get_entity(&self, id: &EntityId) -> DriverResult<Entity> {
        self.entities
            .get(&id.as_str())
            .map(|e| e.value().clone())
            .ok_or_else(|| DriverError::EntityNotFound(id.as_str()))
    }

    async fn delete_entity(&self, id: &EntityId) -> DriverResult<()> {
        self.entities.remove(&id.as_str());
        // Also remove related relationships
        let rels_to_remove: Vec<String> = self
            .relationships
            .iter()
            .filter(|r| r.source_entity_id == *id || r.target_entity_id == *id)
            .map(|r| r.id.as_str())
            .collect();
        for rel_id in rels_to_remove {
            self.relationships.remove(&rel_id);
        }
        Ok(())
    }

    async fn list_entities(
        &self,
        group_id: Option<&str>,
        limit: usize,
    ) -> DriverResult<Vec<Entity>> {
        let entities: Vec<Entity> = self
            .entities
            .iter()
            .filter(|e| match group_id {
                Some(gid) => e.group_id == gid,
                None => true,
            })
            .take(limit)
            .map(|e| e.value().clone())
            .collect();
        Ok(entities)
    }

    // --- Relationship Operations ---

    async fn store_relationship(&self, relationship: &Relationship) -> DriverResult<()> {
        self.relationships
            .insert(relationship.id.as_str(), relationship.clone());
        Ok(())
    }

    async fn get_relationship(&self, id: &RelationshipId) -> DriverResult<Relationship> {
        self.relationships
            .get(&id.as_str())
            .map(|r| r.value().clone())
            .ok_or_else(|| DriverError::RelationshipNotFound(id.as_str()))
    }

    async fn get_entity_relationships(
        &self,
        entity_id: &EntityId,
    ) -> DriverResult<Vec<Relationship>> {
        let rels: Vec<Relationship> = self
            .relationships
            .iter()
            .filter(|r| r.source_entity_id == *entity_id || r.target_entity_id == *entity_id)
            .map(|r| r.value().clone())
            .collect();
        Ok(rels)
    }

    async fn delete_relationship(&self, id: &RelationshipId) -> DriverResult<()> {
        self.relationships.remove(&id.as_str());
        Ok(())
    }

    // --- Episode Operations ---

    async fn store_episode(&self, episode: &Episode) -> DriverResult<()> {
        self.episodes.insert(episode.id.as_str(), episode.clone());
        Ok(())
    }

    async fn get_episode(&self, id: &EpisodeId) -> DriverResult<Episode> {
        self.episodes
            .get(&id.as_str())
            .map(|e| e.value().clone())
            .ok_or_else(|| DriverError::EpisodeNotFound(id.as_str()))
    }

    async fn list_episodes(
        &self,
        group_id: Option<&str>,
        limit: usize,
    ) -> DriverResult<Vec<Episode>> {
        let mut episodes: Vec<Episode> = self
            .episodes
            .iter()
            .filter(|e| match group_id {
                Some(gid) => e.group_id == gid,
                None => true,
            })
            .map(|e| e.value().clone())
            .collect();

        // Sort by created_at descending (most recent first)
        episodes.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        episodes.truncate(limit);
        Ok(episodes)
    }

    // --- Community Operations ---

    async fn store_community(&self, community: &Community) -> DriverResult<()> {
        self.communities
            .insert(community.id.as_str().to_string(), community.clone());
        Ok(())
    }

    async fn get_community(&self, id: &CommunityId) -> DriverResult<Community> {
        self.communities
            .get(id.as_str())
            .map(|c| c.value().clone())
            .ok_or_else(|| DriverError::CommunityNotFound(id.as_str().to_string()))
    }

    async fn get_communities_at_level(
        &self,
        level: CommunityLevel,
    ) -> DriverResult<Vec<Community>> {
        let communities: Vec<Community> = self
            .communities
            .iter()
            .filter(|c| c.level == level)
            .map(|c| c.value().clone())
            .collect();
        Ok(communities)
    }

    async fn get_dirty_communities(&self) -> DriverResult<Vec<Community>> {
        let communities: Vec<Community> = self
            .communities
            .iter()
            .filter(|c| c.is_dirty)
            .map(|c| c.value().clone())
            .collect();
        Ok(communities)
    }

    async fn list_communities(&self, group_id: Option<&str>) -> DriverResult<Vec<Community>> {
        let communities: Vec<Community> = self
            .communities
            .iter()
            .filter(|c| match group_id {
                Some(gid) => c.group_id == gid,
                None => true,
            })
            .map(|c| c.value().clone())
            .collect();
        Ok(communities)
    }

    // --- Search Operations ---

    async fn search_entities_by_vector(
        &self,
        embedding: &[f32],
        k: usize,
        group_id: Option<&str>,
    ) -> DriverResult<Vec<VectorSearchResult>> {
        let mut results: Vec<VectorSearchResult> = self
            .entities
            .iter()
            .filter(|e| match group_id {
                Some(gid) => e.group_id == gid,
                None => true,
            })
            .filter_map(|e| {
                e.name_embedding.as_ref().map(|emb| {
                    let score = cosine_similarity(embedding, emb);
                    VectorSearchResult {
                        entity: e.value().clone(),
                        score,
                    }
                })
            })
            .collect();

        // Sort by score descending
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
        let mut scored: Vec<(Relationship, f64)> = self
            .relationships
            .iter()
            .filter(|r| match group_id {
                Some(gid) => r.group_id == gid,
                None => true,
            })
            .filter_map(|r| {
                r.fact_embedding.as_ref().map(|emb| {
                    let score = cosine_similarity(embedding, emb);
                    (r.value().clone(), score)
                })
            })
            .collect();

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
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut results: Vec<TextSearchResult> = self
            .entities
            .iter()
            .filter(|e| match group_id {
                Some(gid) => e.group_id == gid,
                None => true,
            })
            .filter_map(|e| {
                let name_lower = e.name.to_lowercase();
                let summary_lower = e.summary.to_lowercase();
                let searchable = format!("{} {}", name_lower, summary_lower);

                // Simple BM25-like scoring: count matching terms
                let matching_terms = query_terms
                    .iter()
                    .filter(|term| searchable.contains(*term))
                    .count();

                if matching_terms > 0 {
                    let score = matching_terms as f64 / query_terms.len() as f64;
                    // Boost exact name matches
                    let name_boost = if name_lower.contains(&query_lower) {
                        2.0
                    } else {
                        1.0
                    };
                    Some(TextSearchResult {
                        entity: e.value().clone(),
                        score: score * name_boost,
                    })
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);
        Ok(results)
    }

    // --- Graph Traversal ---

    async fn traverse(
        &self,
        start: &EntityId,
        max_depth: usize,
        group_id: Option<&str>,
    ) -> DriverResult<Subgraph> {
        let mut visited_entities = std::collections::HashSet::new();
        let mut visited_rels = std::collections::HashSet::new();
        let mut queue: std::collections::VecDeque<(EntityId, usize)> =
            std::collections::VecDeque::new();

        queue.push_back((start.clone(), 0));

        let mut subgraph = Subgraph::default();

        while let Some((entity_id, depth)) = queue.pop_front() {
            if depth > max_depth || visited_entities.contains(&entity_id.as_str()) {
                continue;
            }
            visited_entities.insert(entity_id.as_str());

            if let Ok(entity) = self.get_entity(&entity_id).await {
                if let Some(gid) = group_id {
                    if entity.group_id != gid {
                        continue;
                    }
                }
                subgraph.entities.push(entity);
            }

            // Get connected relationships
            if let Ok(rels) = self.get_entity_relationships(&entity_id).await {
                for rel in rels {
                    if visited_rels.contains(&rel.id.as_str()) {
                        continue;
                    }
                    if !rel.is_valid() {
                        continue; // Skip invalidated relationships
                    }
                    visited_rels.insert(rel.id.as_str());
                    subgraph.relationships.push(rel.clone());

                    // Traverse to the other end
                    let next_id = if rel.source_entity_id == entity_id {
                        rel.target_entity_id.clone()
                    } else {
                        rel.source_entity_id.clone()
                    };
                    queue.push_back((next_id, depth + 1));
                }
            }
        }

        Ok(subgraph)
    }

    // --- Temporal Operations ---

    async fn snapshot_at(
        &self,
        timestamp: &DateTime<Utc>,
        group_id: Option<&str>,
    ) -> DriverResult<Subgraph> {
        let entities: Vec<Entity> = self
            .entities
            .iter()
            .filter(|e| {
                let group_match = match group_id {
                    Some(gid) => e.group_id == gid,
                    None => true,
                };
                let time_match = e.created_at <= *timestamp;
                group_match && time_match
            })
            .map(|e| e.value().clone())
            .collect();

        let relationships: Vec<Relationship> = self
            .relationships
            .iter()
            .filter(|r| {
                let group_match = match group_id {
                    Some(gid) => r.group_id == gid,
                    None => true,
                };
                let time_match = r.is_valid_at(timestamp);
                group_match && time_match
            })
            .map(|r| r.value().clone())
            .collect();

        Ok(Subgraph {
            entities,
            relationships,
        })
    }

    // --- Maintenance ---

    async fn stats(&self) -> DriverResult<HashMap<String, usize>> {
        let mut stats = HashMap::new();
        stats.insert("entities".to_string(), self.entities.len());
        stats.insert("relationships".to_string(), self.relationships.len());
        stats.insert("episodes".to_string(), self.episodes.len());
        stats.insert("communities".to_string(), self.communities.len());
        Ok(stats)
    }

    async fn clear(&self) -> DriverResult<()> {
        self.entities.clear();
        self.relationships.clear();
        self.episodes.clear();
        self.communities.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_driver_entity_crud() {
        let driver = MemoryDriver::new();

        let entity = Entity::new("Alice", "Person").with_summary("A researcher");
        driver.store_entity(&entity).await.unwrap();

        let retrieved = driver.get_entity(&entity.id).await.unwrap();
        assert_eq!(retrieved.name, "Alice");
        assert_eq!(retrieved.summary, "A researcher");

        driver.delete_entity(&entity.id).await.unwrap();
        assert!(driver.get_entity(&entity.id).await.is_err());
    }

    #[tokio::test]
    async fn test_memory_driver_relationship_crud() {
        let driver = MemoryDriver::new();

        let alice = Entity::new("Alice", "Person");
        let anthropic = Entity::new("Anthropic", "Organization");
        driver.store_entity(&alice).await.unwrap();
        driver.store_entity(&anthropic).await.unwrap();

        let rel = Relationship::new(
            alice.id.clone(),
            anthropic.id.clone(),
            "WORKS_AT",
            "Alice works at Anthropic",
        );
        driver.store_relationship(&rel).await.unwrap();

        let rels = driver.get_entity_relationships(&alice.id).await.unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].fact, "Alice works at Anthropic");
    }

    #[tokio::test]
    async fn test_memory_driver_vector_search() {
        let driver = MemoryDriver::new();

        let entity1 =
            Entity::new("Machine Learning", "Concept").with_embedding(vec![1.0, 0.0, 0.0]);
        let entity2 = Entity::new("Deep Learning", "Concept").with_embedding(vec![0.9, 0.1, 0.0]);
        let entity3 = Entity::new("Cooking", "Concept").with_embedding(vec![0.0, 0.0, 1.0]);

        driver.store_entity(&entity1).await.unwrap();
        driver.store_entity(&entity2).await.unwrap();
        driver.store_entity(&entity3).await.unwrap();

        let results = driver
            .search_entities_by_vector(&[1.0, 0.0, 0.0], 2, None)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].entity.name, "Machine Learning");
        assert!(results[0].score > results[1].score);
    }

    #[tokio::test]
    async fn test_memory_driver_text_search() {
        let driver = MemoryDriver::new();

        let entity1 =
            Entity::new("Alice Smith", "Person").with_summary("A researcher at Anthropic");
        let entity2 = Entity::new("Bob Jones", "Person").with_summary("A chef in NYC");

        driver.store_entity(&entity1).await.unwrap();
        driver.store_entity(&entity2).await.unwrap();

        let results = driver
            .search_entities_by_text("Alice researcher", 10, None)
            .await
            .unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].entity.name, "Alice Smith");
    }

    #[tokio::test]
    async fn test_memory_driver_traversal() {
        let driver = MemoryDriver::new();

        let alice = Entity::new("Alice", "Person");
        let bob = Entity::new("Bob", "Person");
        let anthropic = Entity::new("Anthropic", "Org");

        driver.store_entity(&alice).await.unwrap();
        driver.store_entity(&bob).await.unwrap();
        driver.store_entity(&anthropic).await.unwrap();

        let rel1 = Relationship::new(
            alice.id.clone(),
            anthropic.id.clone(),
            "WORKS_AT",
            "Alice works at Anthropic",
        );
        let rel2 = Relationship::new(
            bob.id.clone(),
            anthropic.id.clone(),
            "WORKS_AT",
            "Bob works at Anthropic",
        );
        driver.store_relationship(&rel1).await.unwrap();
        driver.store_relationship(&rel2).await.unwrap();

        let subgraph = driver.traverse(&alice.id, 2, None).await.unwrap();
        assert_eq!(subgraph.entities.len(), 3); // Alice, Anthropic, Bob
        assert_eq!(subgraph.relationships.len(), 2);
    }

    #[tokio::test]
    async fn test_memory_driver_temporal_snapshot() {
        let driver = MemoryDriver::new();

        let alice = Entity::new("Alice", "Person");
        driver.store_entity(&alice).await.unwrap();

        let past = Utc::now() - chrono::Duration::days(1);
        let rel = Relationship::new(
            alice.id.clone(),
            EntityId::new(),
            "LIVES_IN",
            "Alice lives in NYC",
        )
        .with_valid_from(past);
        driver.store_relationship(&rel).await.unwrap();

        let snapshot = driver.snapshot_at(&Utc::now(), None).await.unwrap();
        assert_eq!(snapshot.relationships.len(), 1);

        // Snapshot before the relationship was valid
        let very_old = Utc::now() - chrono::Duration::days(365);
        let old_snapshot = driver.snapshot_at(&very_old, None).await.unwrap();
        assert_eq!(old_snapshot.relationships.len(), 0);
    }
}
