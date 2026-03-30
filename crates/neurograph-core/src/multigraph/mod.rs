// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Multi-Graph Memory Architecture (MAGMA-style).
//!
//! Each memory item exists across four orthogonal graph views:
//! - **Semantic** — Embedding similarity edges
//! - **Temporal** — Allen's interval algebra relations
//! - **Causal** — Cause-effect directed edges
//! - **Entity** — Typed entity relationships
//!
//! An intent-aware query router selects which views to traverse,
//! and a fusion engine merges subgraph results using type-aligned
//! Reciprocal Rank Fusion (RRF) with cross-view reinforcement.
//!
//! Reference: "MAGMA represents each memory item across four orthogonal
//! relational graphs, yielding a disentangled representation" (2026).

pub mod causal;
pub mod entity;
pub mod fusion;
pub mod intent;
pub mod semantic;
pub mod temporal;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// Re-export key types
pub use fusion::FusionEngine;
pub use intent::{FusionStrategy, IntentRouter, IntentType, QueryIntent, TraversalPlan};

/// A single memory item that exists across all four graph views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: Uuid,
    pub content: String,
    pub embedding: Vec<f32>,
    pub created_at: DateTime<Utc>,
    pub valid_from: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
    pub importance: f64,
    pub access_count: u64,
    pub last_accessed: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub source_episode: Option<Uuid>,
    pub entity_ids: Vec<Uuid>,
    pub tier: MemoryTier,
}

impl MemoryItem {
    /// Create a new memory item from text content.
    pub fn new(content: String, embedding: Vec<f32>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            content,
            embedding,
            created_at: now,
            valid_from: now,
            valid_until: None,
            importance: 0.5,
            access_count: 0,
            last_accessed: now,
            metadata: HashMap::new(),
            source_episode: None,
            entity_ids: Vec::new(),
            tier: MemoryTier::Episodic,
        }
    }
}

/// Memory tier in the L1–L4 hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryTier {
    /// L1: Current context window, hot cache.
    Working,
    /// L2: Recent interactions, indexed by time.
    Episodic,
    /// L3: Knowledge graph facts (core graph).
    Semantic,
    /// L4: Learned patterns and heuristics.
    Procedural,
}

/// Which graph view to use for retrieval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GraphView {
    Semantic,
    Temporal,
    Causal,
    Entity,
}

/// An edge in any of the four graph views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: Uuid,
    pub source: Uuid,
    pub target: Uuid,
    pub view: GraphView,
    pub relation: String,
    pub weight: f64,
    pub valid_from: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A subgraph result from traversing one graph view.
#[derive(Debug, Clone)]
pub struct SubgraphResult {
    pub view: GraphView,
    pub node_ids: Vec<Uuid>,
    pub scores: Vec<f64>,
    pub edges: Vec<GraphEdge>,
    pub duration_ms: u64,
}

/// Configuration for the multi-graph memory system.
#[derive(Debug, Clone)]
pub struct MultiGraphConfig {
    pub similarity_threshold: f64,
    pub max_neighbors: usize,
    pub temporal_window_days: i64,
    pub causal_confidence_threshold: f64,
    pub enable_offline_mode: bool,
    pub embedding_dim: usize,
}

impl Default for MultiGraphConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.75,
            max_neighbors: 20,
            temporal_window_days: 365,
            causal_confidence_threshold: 0.6,
            enable_offline_mode: true,
            embedding_dim: 128, // Match HashEmbedder default
        }
    }
}

/// The four orthogonal graph views, each stored independently.
pub struct MultiGraphMemory {
    pub semantic_graph: Arc<RwLock<semantic::SemanticGraph>>,
    pub temporal_graph: Arc<RwLock<temporal::TemporalGraph>>,
    pub causal_graph: Arc<RwLock<causal::CausalGraph>>,
    pub entity_graph: Arc<RwLock<entity::EntityGraph>>,
    pub items: Arc<RwLock<HashMap<Uuid, MemoryItem>>>,
    pub intent_router: IntentRouter,
    pub fusion_engine: FusionEngine,
}

impl MultiGraphMemory {
    /// Create a new multi-graph memory system with the given config.
    pub fn new(config: MultiGraphConfig) -> Self {
        Self {
            semantic_graph: Arc::new(RwLock::new(semantic::SemanticGraph::new(&config))),
            temporal_graph: Arc::new(RwLock::new(temporal::TemporalGraph::new(&config))),
            causal_graph: Arc::new(RwLock::new(causal::CausalGraph::new(&config))),
            entity_graph: Arc::new(RwLock::new(entity::EntityGraph::new(&config))),
            items: Arc::new(RwLock::new(HashMap::new())),
            intent_router: IntentRouter::new(),
            fusion_engine: FusionEngine::new(),
        }
    }

    /// Ingest a memory item into all four graph views concurrently.
    pub async fn ingest(&self, item: MemoryItem) -> IngestResult {
        let id = item.id;

        // Store the item
        self.items.write().await.insert(id, item.clone());

        // Fan out to all four graphs concurrently
        let (sem, temp, causal, entity) = tokio::join!(
            self.ingest_semantic(&item),
            self.ingest_temporal(&item),
            self.ingest_causal(&item),
            self.ingest_entity(&item),
        );

        IngestResult {
            item_id: id,
            semantic_edges: sem,
            temporal_edges: temp,
            causal_edges: causal,
            entity_edges: entity,
        }
    }

    /// Intent-aware query: classifies intent, selects views, traverses, fuses.
    pub async fn query(&self, query: &str, opts: QueryOptions) -> QueryResult {
        // Step 1: Classify query intent
        let intent = self.intent_router.classify(query);

        // Step 2: Select which graph views to traverse
        let selected_views = self.intent_router.select_views(&intent);

        // Step 3: Traverse selected views in parallel
        let mut subgraph_results = Vec::new();

        for view in &selected_views {
            let result = match view {
                GraphView::Semantic => {
                    let g = self.semantic_graph.read().await;
                    g.traverse(query, &opts)
                }
                GraphView::Temporal => {
                    let g = self.temporal_graph.read().await;
                    g.traverse(query, &opts)
                }
                GraphView::Causal => {
                    let g = self.causal_graph.read().await;
                    g.traverse(query, &opts)
                }
                GraphView::Entity => {
                    let g = self.entity_graph.read().await;
                    g.traverse(query, &opts)
                }
            };
            subgraph_results.push(result);
        }

        // Step 4: Fuse subgraph results
        self.fusion_engine.fuse(subgraph_results, &intent)
    }

    /// Get statistics about the multi-graph memory.
    pub async fn stats(&self) -> MultiGraphStats {
        let items = self.items.read().await;
        let sem = self.semantic_graph.read().await;
        let temp = self.temporal_graph.read().await;
        let causal = self.causal_graph.read().await;
        let entity = self.entity_graph.read().await;

        MultiGraphStats {
            total_items: items.len(),
            semantic_edges: sem.edge_count(),
            temporal_edges: temp.edge_count(),
            causal_edges: causal.edge_count(),
            entity_edges: entity.edge_count(),
        }
    }

    async fn ingest_semantic(&self, item: &MemoryItem) -> Vec<GraphEdge> {
        let mut graph = self.semantic_graph.write().await;
        graph.add_item(item)
    }

    async fn ingest_temporal(&self, item: &MemoryItem) -> Vec<GraphEdge> {
        let mut graph = self.temporal_graph.write().await;
        graph.add_item(item)
    }

    async fn ingest_causal(&self, item: &MemoryItem) -> Vec<GraphEdge> {
        let mut graph = self.causal_graph.write().await;
        graph.add_item(item)
    }

    async fn ingest_entity(&self, item: &MemoryItem) -> Vec<GraphEdge> {
        let mut graph = self.entity_graph.write().await;
        graph.add_item(item)
    }
}

/// Result of ingesting a memory item.
#[derive(Debug)]
pub struct IngestResult {
    pub item_id: Uuid,
    pub semantic_edges: Vec<GraphEdge>,
    pub temporal_edges: Vec<GraphEdge>,
    pub causal_edges: Vec<GraphEdge>,
    pub entity_edges: Vec<GraphEdge>,
}

/// Options for multi-graph queries.
#[derive(Debug, Clone)]
pub struct QueryOptions {
    pub max_results: usize,
    pub time_point: Option<DateTime<Utc>>,
    pub branch: Option<String>,
    pub budget_usd: Option<f64>,
    pub views: Option<Vec<GraphView>>,
    pub group_id: Option<String>,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            max_results: 10,
            time_point: None,
            branch: None,
            budget_usd: None,
            views: None,
            group_id: None,
        }
    }
}

/// Result from a multi-graph query.
#[derive(Debug)]
pub struct QueryResult {
    pub items: Vec<(Uuid, f64)>,
    pub views_used: Vec<GraphView>,
    pub reasoning_trace: Vec<ReasoningStep>,
}

/// A single step in the reasoning trace (for observability/debugging).
#[derive(Debug, Clone)]
pub struct ReasoningStep {
    pub view: GraphView,
    pub operation: String,
    pub nodes_visited: usize,
    pub duration_ms: u64,
    pub explanation: String,
}

/// Statistics about the multi-graph memory system.
#[derive(Debug)]
pub struct MultiGraphStats {
    pub total_items: usize,
    pub semantic_edges: usize,
    pub temporal_edges: usize,
    pub causal_edges: usize,
    pub entity_edges: usize,
}
