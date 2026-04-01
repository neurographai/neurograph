// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! # NeuroGraph Core
//!
//! The Operating System for AI Knowledge — ingest anything, remember everything,
//! forget intelligently, reason visually.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use neurograph_core::NeuroGraph;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Zero-config: in-memory, local embeddings
//!     let ng = NeuroGraph::builder().build().await?;
//!
//!     // Add knowledge
//!     ng.add_text("Alice works at Anthropic as a researcher").await?;
//!     ng.add_text("Bob founded Anthropic in 2021").await?;
//!
//!     // Query
//!     let result = ng.query("Who works at Anthropic?").await?;
//!     println!("{}", result.answer);
//!
//!     // Time travel
//!     let past = ng.at("2020-01-01").await?;
//!     println!("{}", past.query("Who works at Anthropic?").await?.answer);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │            NeuroGraph (Public API)       │
//! │  add_text · add_json · query · at       │
//! ├─────────────────────────────────────────┤
//! │         Engine (Orchestration)           │
//! │  extraction · dedup · resolution        │
//! ├──────────────┬──────────────────────────┤
//! │  LLM Client  │  Embedder               │
//! │  openai/llama│  openai/local            │
//! ├──────────────┴──────────────────────────┤
//! │         Graph Driver                    │
//! │  memory · embedded · neo4j              │
//! └─────────────────────────────────────────┘
//! ```

pub mod community;
pub mod config;
pub mod drivers;
pub mod embedders;
pub mod engine;
pub mod graph;
pub mod ingestion;
pub mod llm;
pub mod memory;
pub mod multigraph;
pub mod retrieval;
pub mod temporal;
pub mod utils;

// Research Paper Intelligence modules
#[cfg(feature = "pdf")]
pub mod pdf;
pub mod embeddings;
pub mod papers;
pub mod chat;
#[cfg(feature = "server")]
pub mod server;

// Re-export key types at crate root
pub use community::{
    CommunityDetectionResult, LeidenConfig, LeidenDetector, LouvainConfig, LouvainDetector,
};
pub use config::{EmbeddingProvider, NeuroGraphConfig, NeuroGraphConfigBuilder, StorageBackend};
pub use drivers::traits::{DriverError, GraphDriver};
pub use embedders::traits::Embedder;

// Embedding architecture re-exports
pub use embedders::{
    EmbeddingConfig, EmbeddingFactory, EmbeddingModelInfo, EmbeddingRegistry,
    EmbeddingRouter, OpenAICompatibleConfig, OpenAICompatibleEmbedder,
    ApiKeySource, DimensionAligner, EmbeddingMetadata,
    HnswConfig, HnswIndex,
    build_from_toml,
};
pub use graph::{Community, Entity, EntityId, Episode, Relationship, Saga, SagaId};
pub use llm::traits::LlmClient;
pub use temporal::{LogicalClock, TemporalDiff, TemporalSnapshot};

// Multi-graph memory re-exports
pub use memory::{
    ConsolidationConfig, DecayPolicy, MemoryEvolution, PromotionPolicy, RetentionPolicy,
    TieredMemory,
};
pub use multigraph::{
    FusionEngine, GraphEdge, GraphView, IntentRouter, IntentType, MemoryItem, MemoryTier,
    MultiGraphConfig, MultiGraphMemory, QueryOptions as MultiGraphQueryOptions,
};

// Retrieval re-exports (legacy + new)
pub use retrieval::{
    BM25Index, CrossEncoderReranker, DriftResult, DriftSearch, DriftStrategy, PersonalizedPageRank,
    RerankCandidate, RerankResult,
    // P0: MMR, SearchConfig, SearchResults
    MmrReranker, SimilarityMetric,
    ChannelConfig, RerankerType, SearchConfig, SearchMethod,
    Channel, FlatResult, SearchMetadata, SearchResults, ScoredItem,
    // P1: Node Distance, Episode Mentions
    NodeDistanceReranker, EpisodeMentionsReranker, HasEpisodeIds,
};

// LLM re-exports (new P1 types)
pub use llm::{LlmCache, PromptType, TokenTracker, TokenUsage};

use std::sync::Arc;

use drivers::embedded::EmbeddedDriver;
use drivers::memory::MemoryDriver;
use engine::router::QueryRouter;
use graph::schema::GraphSchema;
use ingestion::pipeline::IngestionPipeline;
use utils::concurrency::{ConcurrencyLimiter, CostTracker};

/// Error types for NeuroGraph operations.
#[derive(Debug, thiserror::Error)]
pub enum NeuroGraphError {
    #[error("Driver error: {0}")]
    Driver(#[from] DriverError),

    #[error("LLM error: {0}")]
    Llm(#[from] llm::traits::LlmError),

    #[error("Embedder error: {0}")]
    Embedder(#[from] embedders::traits::EmbedderError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Budget exceeded: ${spent:.4} of ${limit:.4} used")]
    BudgetExceeded { spent: f64, limit: f64 },

    #[error("Temporal error: {0}")]
    Temporal(String),

    #[error("Community detection error: {0}")]
    Community(String),
}

pub type Result<T> = std::result::Result<T, NeuroGraphError>;

/// Query result from NeuroGraph.
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Natural language answer to the query.
    pub answer: String,

    /// Entities relevant to the answer.
    pub entities: Vec<Entity>,

    /// Relationships relevant to the answer.
    pub relationships: Vec<Relationship>,

    /// Communities relevant to the answer.
    pub communities: Vec<Community>,

    /// Confidence score (0.0 - 1.0).
    pub confidence: f64,

    /// Total LLM cost for this query in USD.
    pub cost_usd: f64,

    /// Latency in milliseconds.
    pub latency_ms: u64,
}

impl QueryResult {
    /// Create an empty/placeholder result.
    pub fn empty() -> Self {
        Self {
            answer: String::new(),
            entities: Vec::new(),
            relationships: Vec::new(),
            communities: Vec::new(),
            confidence: 0.0,
            cost_usd: 0.0,
            latency_ms: 0,
        }
    }
}

/// The main NeuroGraph instance.
///
/// This is the primary entry point for all operations.
/// Designed after Cognee's DX simplicity:
/// - `ng.add_text()` — ingest knowledge
/// - `ng.query()` — ask questions
/// - `ng.at()` — time travel
/// - `ng.history()` — see what changed
#[allow(dead_code)]
pub struct NeuroGraph {
    /// Configuration.
    config: NeuroGraphConfig,

    /// Graph storage driver.
    pub(crate) driver: Arc<dyn GraphDriver>,

    /// Embedding provider.
    embedder: Arc<dyn Embedder>,

    /// LLM client for extraction and answer generation.
    llm: Option<Arc<dyn LlmClient>>,

    /// Graph schema registry.
    schema: Arc<parking_lot::RwLock<GraphSchema>>,

    /// Query router for strategy selection.
    router: QueryRouter,

    /// Concurrency limiter for LLM calls.
    limiter: ConcurrencyLimiter,

    /// Cost tracker for budget enforcement.
    cost_tracker: CostTracker,

    /// Multi-graph memory subsystem (optional, enabled via builder).
    multigraph: Option<tokio::sync::RwLock<MultiGraphMemory>>,

    /// Tiered memory subsystem (optional, enabled via builder).
    tiered_memory: Option<parking_lot::RwLock<TieredMemory>>,

    /// Memory evolution engine (optional, enabled via builder).
    memory_evolution: Option<parking_lot::RwLock<MemoryEvolution>>,
}

impl NeuroGraph {
    /// Create a builder for NeuroGraph.
    pub fn builder() -> NeuroGraphBuilder {
        NeuroGraphBuilder::default()
    }

    /// Get the configuration.
    pub fn config(&self) -> &NeuroGraphConfig {
        &self.config
    }

    /// Get the schema registry.
    pub fn schema(&self) -> GraphSchema {
        self.schema.read().clone()
    }

    /// Get the total LLM cost so far.
    pub fn total_cost_usd(&self) -> f64 {
        self.cost_tracker.total_cost_usd()
    }

    /// Get driver statistics.
    pub async fn stats(&self) -> Result<std::collections::HashMap<String, usize>> {
        Ok(self.driver.stats().await?)
    }

    // ─── Ingestion API ────────────────────────────────────────────

    /// Add raw text to the knowledge graph.
    ///
    /// Full pipeline:
    /// 1. Create an Episode (provenance record)
    /// 2. Extract entities and relationships (LLM or regex fallback)
    /// 3. Deduplicate entities against existing graph
    /// 4. Resolve temporal conflicts
    /// 5. Store with embeddings
    pub async fn add_text(&self, text: &str) -> Result<Episode> {
        let pipeline = IngestionPipeline::new(
            self.driver.clone(),
            self.embedder.clone(),
            self.llm.clone(),
            self.config.ontology.clone(),
            self.config.default_group_id.clone(),
        );

        let (episode, result) = pipeline
            .ingest_text(text, "text-input")
            .await
            .map_err(|e| NeuroGraphError::Parse(e.to_string()))?;

        // Track cost
        self.cost_tracker.record(result.cost_usd);

        tracing::info!(
            episode_id = %episode.id,
            entities = result.entities_stored,
            relationships = result.relationships_stored,
            deduped = result.entities_deduplicated,
            cost = result.cost_usd,
            "Text ingested"
        );

        Ok(episode)
    }

    /// Add a JSON object to the knowledge graph.
    pub async fn add_json(&self, data: serde_json::Value) -> Result<Episode> {
        let pipeline = IngestionPipeline::new(
            self.driver.clone(),
            self.embedder.clone(),
            self.llm.clone(),
            self.config.ontology.clone(),
            self.config.default_group_id.clone(),
        );

        let (episode, result) = pipeline
            .ingest_json(&data, "json-input")
            .await
            .map_err(|e| NeuroGraphError::Parse(e.to_string()))?;

        self.cost_tracker.record(result.cost_usd);

        Ok(episode)
    }

    // ─── Entity API ───────────────────────────────────────────────

    /// Store an entity directly.
    pub async fn store_entity(&self, entity: &Entity) -> Result<()> {
        // Generate embedding if not present
        let mut entity = entity.clone();
        if entity.name_embedding.is_none() {
            let embedding = self.embedder.embed_one(&entity.name).await?;
            entity.name_embedding = Some(embedding);
        }

        self.driver.store_entity(&entity).await?;

        // Update schema
        self.schema
            .write()
            .record_entity_type(entity.entity_type.as_str());

        Ok(())
    }

    /// Get an entity by ID.
    pub async fn get_entity(&self, id: &EntityId) -> Result<Entity> {
        Ok(self.driver.get_entity(id).await?)
    }

    /// Search entities by text.
    pub async fn search_entities(&self, query: &str, limit: usize) -> Result<Vec<Entity>> {
        // Try vector search first
        let embedding = self.embedder.embed_one(query).await?;
        let vector_results = self
            .driver
            .search_entities_by_vector(&embedding, limit, None)
            .await?;

        if !vector_results.is_empty() {
            Ok(vector_results.into_iter().map(|r| r.entity).collect())
        } else {
            // Fallback to text search
            let text_results = self
                .driver
                .search_entities_by_text(query, limit, None)
                .await?;
            Ok(text_results.into_iter().map(|r| r.entity).collect())
        }
    }

    // ─── Relationship API ─────────────────────────────────────────

    /// Store a relationship directly.
    pub async fn store_relationship(&self, rel: &Relationship) -> Result<()> {
        let mut rel = rel.clone();
        if rel.fact_embedding.is_none() {
            let embedding = self.embedder.embed_one(&rel.fact).await?;
            rel.fact_embedding = Some(embedding);
        }

        self.driver.store_relationship(&rel).await?;
        self.schema
            .write()
            .record_relationship_type(&rel.relationship_type);
        Ok(())
    }

    /// Get relationships for an entity.
    pub async fn get_relationships(&self, entity_id: &EntityId) -> Result<Vec<Relationship>> {
        Ok(self.driver.get_entity_relationships(entity_id).await?)
    }

    // ─── Query API ────────────────────────────────────────────────

    /// Query the knowledge graph with natural language.
    ///
    /// Routes to the optimal strategy based on query classification:
    /// - Local: direct entity lookup (fast, cheap)
    /// - Global: community summary map-reduce (Sprint 3)
    /// - Temporal: time-aware traversal (Sprint 3)
    pub async fn query(&self, question: &str) -> Result<QueryResult> {
        let result = self
            .router
            .execute(
                question,
                self.driver.clone(),
                self.embedder.clone(),
                self.llm.clone(),
                Some(self.config.default_group_id.clone()),
                self.config.budget_usd,
            )
            .await
            .map_err(|e| NeuroGraphError::Query(e.to_string()))?;

        self.cost_tracker.record(result.cost_usd);

        Ok(result)
    }

    // ─── Temporal API ─────────────────────────────────────────────

    /// Get a temporal snapshot of the graph at a specific date.
    ///
    /// Parses the date string and returns a `TemporalView` that
    /// answers queries as if the knowledge graph only contained
    /// facts valid at that time.
    ///
    /// Supported date formats:
    /// - `"2025-01-15"` (ISO date)
    /// - `"2025-01-15T10:30:00Z"` (ISO datetime)
    /// - `"2025"` (year → January 1)
    pub async fn at(&self, date: &str) -> Result<TemporalView> {
        let temporal_mgr = temporal::TemporalManager::new(self.driver.clone());
        let timestamp = temporal::TemporalManager::parse_date(date)
            .map_err(|e: temporal::TemporalError| NeuroGraphError::Temporal(e.to_string()))?;
        let snapshot = temporal_mgr
            .snapshot_at(timestamp, None)
            .await
            .map_err(|e: temporal::TemporalError| NeuroGraphError::Temporal(e.to_string()))?;
        Ok(TemporalView {
            snapshot,
            driver: self.driver.clone(),
        })
    }

    /// Add text to the knowledge graph with an explicit timestamp.
    ///
    /// Like `add_text`, but sets the episode and entity timestamps
    /// to the provided date rather than `Utc::now()`.
    ///
    /// ```rust,no_run
    /// # use neurograph_core::NeuroGraph;
    /// # async fn example() -> anyhow::Result<()> {
    /// let ng = NeuroGraph::builder().build().await?;
    /// ng.add_text_at("Alice works at Google", "2023-01-15").await?;
    /// ng.add_text_at("Alice works at Anthropic", "2025-06-15").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_text_at(&self, text: &str, date: &str) -> Result<Episode> {
        let timestamp = temporal::TemporalManager::parse_date(date)
            .map_err(|e: temporal::TemporalError| NeuroGraphError::Temporal(e.to_string()))?;

        let pipeline = ingestion::pipeline::IngestionPipeline::new(
            self.driver.clone(),
            self.embedder.clone(),
            self.llm.clone(),
            self.config.ontology.clone(),
            self.config.default_group_id.clone(),
        );

        let (mut episode, result) = pipeline
            .ingest_text(text, "text-input")
            .await
            .map_err(|e| NeuroGraphError::Parse(e.to_string()))?;

        // Override timestamps with the provided date
        episode.created_at = timestamp;

        self.cost_tracker.record(result.cost_usd);

        tracing::info!(
            episode_id = %episode.id,
            timestamp = %timestamp,
            entities = result.entities_stored,
            "Text ingested at specific date"
        );

        Ok(episode)
    }

    /// Get the history of relationships for a named entity.
    ///
    /// Returns all relationships (including invalidated ones) associated
    /// with the first entity whose name matches the query.
    ///
    /// ```rust,no_run
    /// # use neurograph_core::NeuroGraph;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let ng = NeuroGraph::builder().build().await?;
    /// let history = ng.entity_history("Alice").await?;
    /// for rel in &history {
    ///     println!("{} (valid: {:?} -> {:?})", rel.fact, rel.valid_from, rel.valid_until);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn entity_history(&self, entity_name: &str) -> Result<Vec<graph::Relationship>> {
        // Find entity by name
        let entities = self
            .driver
            .search_entities_by_text(entity_name, 1, None)
            .await?;

        let entity = entities
            .into_iter()
            .next()
            .ok_or_else(|| NeuroGraphError::Query(format!("Entity '{}' not found", entity_name)))?;

        // Get ALL relationships (including invalidated ones)
        let mut rels = self
            .driver
            .get_entity_relationships(&entity.entity.id)
            .await?;

        // Sort by valid_from ascending (chronological order)
        rels.sort_by_key(|r| r.valid_from);

        Ok(rels)
    }

    /// Show what changed between two dates.
    ///
    /// Returns entities added, modified, and relationships invalidated
    /// in the given time window.
    pub async fn what_changed(&self, from: &str, to: &str) -> Result<TemporalDiff> {
        let temporal_mgr = temporal::TemporalManager::new(self.driver.clone());
        let from_ts = temporal::TemporalManager::parse_date(from)
            .map_err(|e: temporal::TemporalError| NeuroGraphError::Temporal(e.to_string()))?;
        let to_ts = temporal::TemporalManager::parse_date(to)
            .map_err(|e: temporal::TemporalError| NeuroGraphError::Temporal(e.to_string()))?;
        temporal_mgr
            .what_changed(from_ts, to_ts, None)
            .await
            .map_err(|e: temporal::TemporalError| NeuroGraphError::Temporal(e.to_string()))
    }

    /// Build a timeline of all events for visualization (G6 Timebar).
    pub async fn build_timeline(&self) -> Result<Vec<temporal::manager::TimelineEvent>> {
        let temporal_mgr = temporal::TemporalManager::new(self.driver.clone());
        temporal_mgr
            .build_timeline(None)
            .await
            .map_err(|e: temporal::TemporalError| NeuroGraphError::Temporal(e.to_string()))
    }

    // ─── Community API ───────────────────────────────────────────

    /// Run Louvain community detection on the entire graph.
    ///
    /// Detects clusters of related entities and stores
    /// them as `Community` objects in the driver.
    pub async fn detect_communities(&self) -> Result<CommunityDetectionResult> {
        let detector = LouvainDetector::new();
        detector
            .detect(self.driver.as_ref(), None)
            .await
            .map_err(|e: community::CommunityError| NeuroGraphError::Community(e.to_string()))
    }

    /// Run Louvain community detection with custom configuration.
    pub async fn detect_communities_with(
        &self,
        config: LouvainConfig,
    ) -> Result<CommunityDetectionResult> {
        let detector = LouvainDetector::with_config(config);
        detector
            .detect(self.driver.as_ref(), None)
            .await
            .map_err(|e: community::CommunityError| NeuroGraphError::Community(e.to_string()))
    }

    /// Summarize all communities that need summarization.
    ///
    /// Uses LLM if available, otherwise falls back to rule-based summaries.
    pub async fn summarize_communities(&self) -> Result<Vec<community::CommunitySummaryResult>> {
        let summarizer = community::CommunitySummarizer::new(self.driver.clone(), self.llm.clone());
        summarizer
            .summarize_all(None)
            .await
            .map_err(|e: community::SummarizerError| NeuroGraphError::Community(e.to_string()))
    }

    /// Update community assignments after ingestion.
    pub async fn update_communities(
        &self,
        entity_ids: &[EntityId],
    ) -> Result<community::IncrementalUpdateResult> {
        let updater = community::IncrementalCommunityUpdater::new(self.driver.clone());
        updater
            .update_after_ingestion(entity_ids, None)
            .await
            .map_err(|e: community::IncrementalError| NeuroGraphError::Community(e.to_string()))
    }

    // ─── Graph Operations ─────────────────────────────────────────

    /// Traverse the graph from a starting entity (BFS).
    pub async fn traverse(
        &self,
        entity_id: &EntityId,
        max_depth: usize,
    ) -> Result<drivers::Subgraph> {
        Ok(self.driver.traverse(entity_id, max_depth, None).await?)
    }

    /// Clear all data (use with caution!).
    pub async fn clear(&self) -> Result<()> {
        self.driver.clear().await?;
        *self.schema.write() = GraphSchema::new(self.config.ontology.clone());
        self.cost_tracker.reset();
        Ok(())
    }

    // ─── Multi-Graph Memory API ──────────────────────────────────

    /// Check if multi-graph memory is enabled.
    pub fn has_multigraph(&self) -> bool {
        self.multigraph.is_some()
    }

    /// Check if tiered memory is enabled.
    pub fn has_tiered_memory(&self) -> bool {
        self.tiered_memory.is_some()
    }

    /// Remember a piece of text in the tiered memory system (enters at L1 Working).
    ///
    /// ```rust,no_run
    /// # use neurograph_core::NeuroGraph;
    /// # async fn example() -> anyhow::Result<()> {
    /// let ng = NeuroGraph::builder().tiered_memory().build().await?;
    /// let id = ng.remember("Alice works at Anthropic");
    /// # Ok(())
    /// # }
    /// ```
    pub fn remember(&self, content: &str) -> Option<uuid::Uuid> {
        self.tiered_memory
            .as_ref()
            .map(|tm| tm.write().remember(content.to_string()))
    }

    /// Search tiered memory for matching content.
    pub fn recall(&self, query: &str, max_results: usize) -> Vec<String> {
        self.tiered_memory
            .as_ref()
            .map(|tm| {
                tm.read()
                    .search(query, max_results)
                    .into_iter()
                    .map(|item| item.content.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Run maintenance on the tiered memory (promotions, demotions, evictions).
    pub fn maintain_memory(&self) -> Option<memory::tiered::MaintenanceReport> {
        self.tiered_memory.as_ref().map(|tm| tm.write().maintain())
    }

    /// Get tiered memory statistics.
    pub fn tiered_stats(&self) -> Option<memory::tiered::TieredStats> {
        self.tiered_memory.as_ref().map(|tm| tm.read().stats())
    }

    /// Add a memory item to the multi-graph memory and index across all views.
    pub async fn multigraph_ingest(&self, content: &str, embedding: Vec<f32>) {
        if let Some(mg) = &self.multigraph {
            let item = multigraph::MemoryItem::new(content.to_string(), embedding);
            mg.write().await.ingest(item).await;
        }
    }

    /// Query the multi-graph memory with intent-aware routing.
    pub async fn multigraph_query(
        &self,
        query: &str,
        max_results: usize,
    ) -> Option<multigraph::QueryResult> {
        if let Some(mg) = &self.multigraph {
            let opts = multigraph::QueryOptions {
                max_results,
                ..Default::default()
            };
            Some(mg.read().await.query(query, opts).await)
        } else {
            None
        }
    }

    /// Run a memory evolution cycle (scoring, decay, RL forgetting, consolidation).
    pub fn evolve_memory(
        &self,
        items: &mut Vec<memory::evolution::EvolvableItem>,
    ) -> Option<memory::evolution::EvolutionResult> {
        self.memory_evolution
            .as_ref()
            .map(|evo| evo.write().evolve(items))
    }
}

impl std::fmt::Debug for NeuroGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NeuroGraph")
            .field("name", &self.config.name)
            .field("driver", &self.driver.name())
            .field("embedder", &self.embedder.model_name())
            .field("cost_usd", &self.cost_tracker.total_cost_usd())
            .finish()
    }
}

/// A temporal view of the knowledge graph at a specific point in time.
///
/// Contains the snapshot data and answers queries as if the graph only
/// contained facts valid at that timestamp.
pub struct TemporalView {
    /// The point-in-time snapshot.
    pub snapshot: TemporalSnapshot,
    /// The underlying driver for queries (used by query()).
    #[allow(dead_code)]
    driver: Arc<dyn GraphDriver>,
}

impl TemporalView {
    /// Get the entities in this temporal view.
    pub fn entities(&self) -> &[Entity] {
        &self.snapshot.entities
    }

    /// Get the relationships valid at this point in time.
    pub fn relationships(&self) -> &[Relationship] {
        &self.snapshot.relationships
    }

    /// Get the entity count at this point in time.
    pub fn entity_count(&self) -> usize {
        self.snapshot.entity_count
    }

    /// Get the relationship count at this point in time.
    pub fn relationship_count(&self) -> usize {
        self.snapshot.relationship_count
    }

    /// Query this temporal snapshot.
    pub async fn query(&self, _question: &str) -> Result<QueryResult> {
        // TODO: Route query through the temporal snapshot's entity/relationship set
        Ok(QueryResult::empty())
    }
}

/// Builder for NeuroGraph instances.
#[derive(Default)]
pub struct NeuroGraphBuilder {
    config: Option<NeuroGraphConfig>,
    config_builder: Option<NeuroGraphConfigBuilder>,
    driver: Option<Arc<dyn GraphDriver>>,
    embedder: Option<Arc<dyn Embedder>>,
    enable_multigraph: bool,
    enable_tiered_memory: bool,
    enable_evolution: bool,
    multigraph_config: Option<MultiGraphConfig>,
    promotion_policy: Option<PromotionPolicy>,
    decay_policy: Option<DecayPolicy>,
}

impl NeuroGraphBuilder {
    /// Set a complete configuration.
    pub fn config(mut self, config: NeuroGraphConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Use in-memory storage (default).
    pub fn memory(mut self) -> Self {
        let cb = self.config_builder.take().unwrap_or_default();
        self.config_builder = Some(cb.memory());
        self
    }

    /// Use embedded (sled) persistent storage.
    pub fn embedded(mut self, path: impl Into<String>) -> Self {
        let cb = self.config_builder.take().unwrap_or_default();
        self.config_builder = Some(cb.embedded(path));
        self
    }

    /// Set graph name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        let cb = self.config_builder.take().unwrap_or_default();
        self.config_builder = Some(cb.name(name));
        self
    }

    /// Set budget limit in USD.
    pub fn budget(mut self, usd: f64) -> Self {
        let cb = self.config_builder.take().unwrap_or_default();
        self.config_builder = Some(cb.budget(usd));
        self
    }

    /// Use a custom driver implementation.
    pub fn driver(mut self, driver: Arc<dyn GraphDriver>) -> Self {
        self.driver = Some(driver);
        self
    }

    /// Use a custom embedder implementation.
    pub fn embedder(mut self, embedder: Arc<dyn Embedder>) -> Self {
        self.embedder = Some(embedder);
        self
    }

    /// Use OpenAI text-embedding-3-small (requires `OPENAI_API_KEY`).
    pub fn openai_embeddings(self) -> Self {
        let cb = self.config_builder.unwrap_or_default().openai_embeddings();
        Self { config_builder: Some(cb), ..self }
    }

    /// Use a specific OpenAI embedding model.
    pub fn openai_embeddings_model(self, model: &str) -> Self {
        let cb = self.config_builder.unwrap_or_default().openai_embeddings_model(model);
        Self { config_builder: Some(cb), ..self }
    }

    /// Use Google Gemini text-embedding-004 (free tier, requires `GEMINI_API_KEY`).
    pub fn gemini_embeddings(self) -> Self {
        let cb = self.config_builder.unwrap_or_default().gemini_embeddings();
        Self { config_builder: Some(cb), ..self }
    }

    /// Use Cohere embed-v4.0 (requires `COHERE_API_KEY`).
    pub fn cohere_embeddings(self) -> Self {
        let cb = self.config_builder.unwrap_or_default().cohere_embeddings();
        Self { config_builder: Some(cb), ..self }
    }

    /// Use Voyage AI voyage-3-large (requires `VOYAGE_API_KEY`).
    pub fn voyage_embeddings(self) -> Self {
        let cb = self.config_builder.unwrap_or_default().voyage_embeddings();
        Self { config_builder: Some(cb), ..self }
    }

    /// Use Jina AI jina-embeddings-v3 (requires `JINA_API_KEY`).
    pub fn jina_embeddings(self) -> Self {
        let cb = self.config_builder.unwrap_or_default().jina_embeddings();
        Self { config_builder: Some(cb), ..self }
    }

    /// Use Mistral mistral-embed (requires `MISTRAL_API_KEY`).
    pub fn mistral_embeddings(self) -> Self {
        let cb = self.config_builder.unwrap_or_default().mistral_embeddings();
        Self { config_builder: Some(cb), ..self }
    }

    /// Use Ollama local embeddings with the given model.
    pub fn ollama_embeddings(self, model: &str) -> Self {
        let cb = self.config_builder.unwrap_or_default().ollama_embeddings(model);
        Self { config_builder: Some(cb), ..self }
    }

    /// Use hash-based embeddings (zero-cost, offline, deterministic).
    pub fn hash_embeddings(self) -> Self {
        let cb = self.config_builder.unwrap_or_default().local_embeddings();
        Self { config_builder: Some(cb), ..self }
    }

    /// Use any OpenAI-compatible embedding endpoint.
    pub fn custom_embeddings(mut self, config: OpenAICompatibleConfig) -> Self {
        let cb = self.config_builder.take().unwrap_or_default().embedding(
            EmbeddingProvider::Custom {
                base_url: config.base_url.clone(),
                model: config.model.clone(),
                api_key_env: match &config.api_key {
                    embedders::openai_compatible::ApiKeySource::Env(k) => k.clone(),
                    _ => String::new(),
                },
                dimensions: config.model_info.dimensions,
            },
        );
        self.config_builder = Some(cb);
        self
    }

    /// Load embedding provider from a TOML config file.
    ///
    /// The TOML file defines providers and the active selection.
    /// New models can be added without any code changes.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use neurograph_core::NeuroGraph;
    /// # async fn example() -> anyhow::Result<()> {
    /// let ng = NeuroGraph::builder()
    ///     .embeddings_from_config("neurograph.toml")
    ///     .build().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn embeddings_from_config(mut self, path: impl AsRef<std::path::Path>) -> Self {
        match embedders::config_file::build_from_toml(path) {
            Ok(embedder) => {
                self.embedder = Some(embedder);
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to load embedding config from TOML");
            }
        }
        self
    }

    /// Enable multi-graph memory subsystem.
    pub fn multigraph(mut self) -> Self {
        self.enable_multigraph = true;
        self
    }

    /// Enable multi-graph memory with custom config.
    pub fn multigraph_with(mut self, config: MultiGraphConfig) -> Self {
        self.enable_multigraph = true;
        self.multigraph_config = Some(config);
        self
    }

    /// Enable tiered memory (L1–L4) subsystem.
    pub fn tiered_memory(mut self) -> Self {
        self.enable_tiered_memory = true;
        self
    }

    /// Enable tiered memory with custom promotion policy.
    pub fn tiered_memory_with(mut self, policy: PromotionPolicy) -> Self {
        self.enable_tiered_memory = true;
        self.promotion_policy = Some(policy);
        self
    }

    /// Enable memory evolution (RL-guided forgetting).
    pub fn evolution(mut self) -> Self {
        self.enable_evolution = true;
        self
    }

    /// Enable memory evolution with custom decay policy.
    pub fn evolution_with(mut self, policy: DecayPolicy) -> Self {
        self.enable_evolution = true;
        self.decay_policy = Some(policy);
        self
    }

    /// Enable all v0.2 subsystems (multi-graph + tiered memory + evolution).
    pub fn full(self) -> Self {
        self.multigraph().tiered_memory().evolution()
    }

    /// Build the NeuroGraph instance.
    pub async fn build(self) -> Result<NeuroGraph> {
        // Resolve configuration
        let config = if let Some(config) = self.config {
            config
        } else if let Some(builder) = self.config_builder {
            builder.build()
        } else {
            NeuroGraphConfig::zero_config()
        };

        // Initialize tracing if enabled
        if config.enable_tracing {
            let _ = tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
                )
                .try_init();
        }

        // Create driver
        let driver: Arc<dyn GraphDriver> = if let Some(driver) = self.driver {
            driver
        } else {
            match &config.storage {
                StorageBackend::Memory => Arc::new(MemoryDriver::new()),
                StorageBackend::Embedded { path } => {
                    let driver = EmbeddedDriver::open(path)
                        .map_err(|e| NeuroGraphError::Config(e.to_string()))?;
                    Arc::new(driver)
                }
            }
        };

        // Create embedder via universal EmbeddingFactory
        let embedder: Arc<dyn Embedder> = if let Some(embedder) = self.embedder {
            embedder
        } else {
            let embed_config = match &config.embedding {
                EmbeddingProvider::Local => EmbeddingConfig::Hash { dimensions: 384 },
                EmbeddingProvider::OpenAi { model } => {
                    EmbeddingConfig::OpenAIModel(model.clone())
                }
                EmbeddingProvider::Gemini { model } => {
                    EmbeddingConfig::GeminiModel(model.clone())
                }
                EmbeddingProvider::Cohere => EmbeddingConfig::Cohere,
                EmbeddingProvider::Voyage => EmbeddingConfig::Voyage,
                EmbeddingProvider::Jina => EmbeddingConfig::Jina,
                EmbeddingProvider::Mistral => EmbeddingConfig::Mistral,
                EmbeddingProvider::Ollama { model } => {
                    EmbeddingConfig::Ollama(model.clone())
                }
                EmbeddingProvider::Custom {
                    base_url,
                    model,
                    api_key_env,
                    dimensions,
                } => EmbeddingConfig::Custom(
                    embedders::providers::EmbeddingRegistry::custom_openai_compatible(
                        base_url, model, api_key_env, *dimensions,
                    ),
                ),
            };
            EmbeddingFactory::build(&embed_config)
                .map_err(|e| NeuroGraphError::Config(e.to_string()))?
        };

        // Create schema
        let schema = Arc::new(parking_lot::RwLock::new(GraphSchema::new(
            config.ontology.clone(),
        )));

        // Create limiter and tracker
        let limiter = ConcurrencyLimiter::new(config.max_concurrent_llm);
        let cost_tracker = CostTracker::new(config.budget_usd);

        // Create LLM client (optional)
        let llm: Option<Arc<dyn LlmClient>> = if std::env::var("OPENAI_API_KEY").is_ok() {
            match llm::openai::OpenAiClient::new(config.llm.clone()) {
                Ok(client) => Some(Arc::new(client)),
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to create OpenAI client, LLM features disabled");
                    None
                }
            }
        } else {
            tracing::info!("No OPENAI_API_KEY set, LLM features disabled (regex extraction only)");
            None
        };

        let router = QueryRouter::new();

        tracing::info!(
            name = %config.name,
            driver = driver.name(),
            embedder = embedder.model_name(),
            llm = llm.as_ref().map(|l| l.model_name()).unwrap_or("none"),
            "NeuroGraph initialized"
        );

        // Initialize optional subsystems
        let multigraph = if self.enable_multigraph {
            let mg_config = self.multigraph_config.unwrap_or_default();
            tracing::info!("Multi-graph memory enabled");
            Some(tokio::sync::RwLock::new(MultiGraphMemory::new(mg_config)))
        } else {
            None
        };

        let tiered_memory = if self.enable_tiered_memory {
            let policy = self.promotion_policy.unwrap_or_default();
            tracing::info!("Tiered memory (L1–L4) enabled");
            Some(parking_lot::RwLock::new(TieredMemory::with_policies(
                policy,
                ConsolidationConfig::default(),
            )))
        } else {
            None
        };

        let memory_evolution = if self.enable_evolution {
            let decay = self.decay_policy.unwrap_or_default();
            tracing::info!("Memory evolution (RL forgetting) enabled");
            Some(parking_lot::RwLock::new(MemoryEvolution::with_policies(
                decay,
                RetentionPolicy::default(),
            )))
        } else {
            None
        };

        Ok(NeuroGraph {
            config,
            driver,
            embedder,
            llm,
            schema,
            router,
            limiter,
            cost_tracker,
            multigraph,
            tiered_memory,
            memory_evolution,
        })
    }
}
