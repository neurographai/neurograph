// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Driver trait definition — the abstraction layer for graph storage backends.
//!
//! Influenced by Graphiti's `GraphDriver` ABC (driver.py):
//! - `execute_query()`, `session()`, `build_indices_and_constraints()`
//! - Provider-agnostic operations for nodes and edges
//!
//! Our trait extends this with:
//! - Native vector search (embedding-based similarity)
//! - Community storage and retrieval
//! - Temporal filtering at the driver level
//! - Batch operations for efficiency

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::graph::{Community, CommunityId, CommunityLevel, Entity, EntityId, Episode, EpisodeId, Relationship, RelationshipId};

/// Errors that can occur during driver operations.
#[derive(Debug, thiserror::Error)]
pub enum DriverError {
    #[error("Entity not found: {0}")]
    EntityNotFound(String),

    #[error("Relationship not found: {0}")]
    RelationshipNotFound(String),

    #[error("Episode not found: {0}")]
    EpisodeNotFound(String),

    #[error("Community not found: {0}")]
    CommunityNotFound(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Index error: {0}")]
    IndexError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Query error: {0}")]
    QueryError(String),
}

/// Result type for driver operations.
pub type DriverResult<T> = Result<T, DriverError>;

/// Configuration for a graph driver.
#[derive(Debug, Clone)]
pub struct GraphDriverConfig {
    /// Path for persistent storage (if applicable).
    pub storage_path: Option<String>,

    /// Maximum number of concurrent connections.
    pub max_connections: usize,

    /// Connection URI (for remote drivers like Neo4j).
    pub uri: Option<String>,

    /// Authentication username.
    pub username: Option<String>,

    /// Authentication password.
    pub password: Option<String>,
}

impl Default for GraphDriverConfig {
    fn default() -> Self {
        Self {
            storage_path: None,
            max_connections: 10,
            uri: None,
            username: None,
            password: None,
        }
    }
}

/// A subgraph result from traversal operations.
#[derive(Debug, Clone, Default)]
pub struct Subgraph {
    /// Entities in the subgraph.
    pub entities: Vec<Entity>,
    /// Relationships in the subgraph.
    pub relationships: Vec<Relationship>,
}

/// Vector search result with relevance score.
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    /// The entity found.
    pub entity: Entity,
    /// Cosine similarity score (0.0 - 1.0).
    pub score: f64,
}

/// Text search result with relevance score.
#[derive(Debug, Clone)]
pub struct TextSearchResult {
    /// The entity found.
    pub entity: Entity,
    /// BM25-style relevance score.
    pub score: f64,
}

/// The core driver trait for graph storage backends.
///
/// Every storage backend (in-memory, sled, Neo4j, FalkorDB, Kuzu)
/// implements this trait. This is the single abstraction point —
/// all engine logic goes through this trait.
///
/// Design decision: We use `async_trait` because all real-world
/// drivers need async I/O. The in-memory driver simply wraps sync
/// operations in async for compatibility.
#[async_trait]
pub trait GraphDriver: Send + Sync {
    /// Get the driver name for logging/debugging.
    fn name(&self) -> &str;

    // --- Entity Operations ---

    /// Store an entity node. Upserts if ID already exists.
    async fn store_entity(&self, entity: &Entity) -> DriverResult<()>;

    /// Store multiple entities in a batch.
    async fn store_entities(&self, entities: &[Entity]) -> DriverResult<()> {
        for entity in entities {
            self.store_entity(entity).await?;
        }
        Ok(())
    }

    /// Retrieve an entity by its ID.
    async fn get_entity(&self, id: &EntityId) -> DriverResult<Entity>;

    /// Retrieve multiple entities by their IDs.
    async fn get_entities(&self, ids: &[EntityId]) -> DriverResult<Vec<Entity>> {
        let mut results = Vec::with_capacity(ids.len());
        for id in ids {
            match self.get_entity(id).await {
                Ok(entity) => results.push(entity),
                Err(DriverError::EntityNotFound(_)) => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(results)
    }

    /// Delete an entity by its ID.
    async fn delete_entity(&self, id: &EntityId) -> DriverResult<()>;

    /// List all entities, optionally filtered by group ID.
    async fn list_entities(&self, group_id: Option<&str>, limit: usize) -> DriverResult<Vec<Entity>>;

    // --- Relationship Operations ---

    /// Store a relationship edge.
    async fn store_relationship(&self, relationship: &Relationship) -> DriverResult<()>;

    /// Store multiple relationships in a batch.
    async fn store_relationships(&self, relationships: &[Relationship]) -> DriverResult<()> {
        for rel in relationships {
            self.store_relationship(rel).await?;
        }
        Ok(())
    }

    /// Retrieve a relationship by its ID.
    async fn get_relationship(&self, id: &RelationshipId) -> DriverResult<Relationship>;

    /// Get all relationships for an entity (both directions).
    async fn get_entity_relationships(&self, entity_id: &EntityId) -> DriverResult<Vec<Relationship>>;

    /// Delete a relationship by its ID.
    async fn delete_relationship(&self, id: &RelationshipId) -> DriverResult<()>;

    // --- Episode Operations ---

    /// Store an episode (provenance record).
    async fn store_episode(&self, episode: &Episode) -> DriverResult<()>;

    /// Retrieve an episode by its ID.
    async fn get_episode(&self, id: &EpisodeId) -> DriverResult<Episode>;

    /// List recent episodes.
    async fn list_episodes(&self, group_id: Option<&str>, limit: usize) -> DriverResult<Vec<Episode>>;

    // --- Community Operations ---

    /// Store a community.
    async fn store_community(&self, community: &Community) -> DriverResult<()>;

    /// Retrieve a community by its ID.
    async fn get_community(&self, id: &CommunityId) -> DriverResult<Community>;

    /// Get all communities at a specific hierarchy level.
    async fn get_communities_at_level(&self, level: CommunityLevel) -> DriverResult<Vec<Community>>;

    /// Get all dirty (need re-summarization) communities.
    async fn get_dirty_communities(&self) -> DriverResult<Vec<Community>>;

    /// List all communities, optionally filtered by group ID.
    async fn list_communities(&self, group_id: Option<&str>) -> DriverResult<Vec<Community>>;

    // --- Search Operations ---

    /// Vector similarity search on entity name embeddings.
    async fn search_entities_by_vector(
        &self,
        embedding: &[f32],
        k: usize,
        group_id: Option<&str>,
    ) -> DriverResult<Vec<VectorSearchResult>>;

    /// Vector similarity search on relationship fact embeddings.
    async fn search_relationships_by_vector(
        &self,
        embedding: &[f32],
        k: usize,
        group_id: Option<&str>,
    ) -> DriverResult<Vec<Relationship>>;

    /// Full-text search on entity names and summaries.
    async fn search_entities_by_text(
        &self,
        query: &str,
        limit: usize,
        group_id: Option<&str>,
    ) -> DriverResult<Vec<TextSearchResult>>;

    // --- Graph Traversal ---

    /// BFS/DFS traversal from a starting entity.
    async fn traverse(
        &self,
        start: &EntityId,
        max_depth: usize,
        group_id: Option<&str>,
    ) -> DriverResult<Subgraph>;

    // --- Temporal Operations ---

    /// Get all entities and relationships valid at a specific timestamp.
    async fn snapshot_at(
        &self,
        timestamp: &DateTime<Utc>,
        group_id: Option<&str>,
    ) -> DriverResult<Subgraph>;

    // --- Maintenance ---

    /// Get summary statistics.
    async fn stats(&self) -> DriverResult<HashMap<String, usize>>;

    /// Clear all data (for testing).
    async fn clear(&self) -> DriverResult<()>;
}
