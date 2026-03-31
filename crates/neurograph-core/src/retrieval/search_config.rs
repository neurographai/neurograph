// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Composable search configuration system.
//!
//! Modeled after Graphiti's `SearchConfig` but made Rust-idiomatic with
//! builder pattern and per-channel (entity/relationship/episode/community)
//! independent configuration. Closes a critical gap where Graphiti has
//! 15+ pre-built search recipes with per-type search method and reranker
//! selection.
//!
//! # Example
//!
//! ```rust
//! use neurograph_core::retrieval::search_config::*;
//!
//! // Build a custom config
//! let config = SearchConfig::builder()
//!     .entities(ChannelConfig {
//!         enabled: true,
//!         methods: vec![SearchMethod::Cosine, SearchMethod::Bm25],
//!         reranker: RerankerType::Mmr { lambda: 0.7 },
//!         limit: 10,
//!         ..Default::default()
//!     })
//!     .communities(ChannelConfig {
//!         enabled: true,
//!         methods: vec![SearchMethod::Cosine],
//!         reranker: RerankerType::Rrf,
//!         limit: 3,
//!         ..Default::default()
//!     })
//!     .build();
//! ```

use serde::{Deserialize, Serialize};

// ─── Search Methods ───────────────────────────────────────────────────

/// Available search methods for candidate retrieval.
///
/// Each channel can use one or more of these methods. Results from
/// multiple methods are merged before reranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SearchMethod {
    /// Vector similarity search (cosine on embeddings).
    Cosine,
    /// Full-text search via the BM25 inverted index.
    Bm25,
    /// Breadth-first graph traversal from seed nodes.
    Bfs,
    /// Personalized PageRank from query-relevant seeds (NeuroGraph unique).
    Ppr,
    /// DRIFT — dynamic reasoning with flexible traversal (NeuroGraph unique).
    DriftAdaptive,
}

// ─── Reranker Selection ───────────────────────────────────────────────

/// Reranker strategy applied after candidate retrieval.
///
/// Each search channel can independently choose its reranker.
/// This matches Graphiti's per-channel reranker architecture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RerankerType {
    /// Reciprocal Rank Fusion — merges multiple ranked lists.
    Rrf,
    /// Maximal Marginal Relevance — diversity-aware reranking.
    Mmr {
        /// Trade-off: 1.0 = pure relevance, 0.0 = pure diversity.
        lambda: f32,
    },
    /// LLM-based cross-encoder scoring (most accurate, highest cost).
    CrossEncoder {
        /// Model name for the cross-encoder.
        model: String,
    },
    /// Rerank by graph distance from a center node.
    NodeDistance {
        /// The focal node to measure distances from.
        center_node_id: Option<String>,
    },
    /// Rerank by number of episode (provenance) mentions.
    EpisodeMentions,
    /// No reranking — return raw scores from search methods.
    None,
}

impl Default for RerankerType {
    fn default() -> Self {
        Self::Rrf
    }
}

// ─── Per-Channel Config ───────────────────────────────────────────────

/// Configuration for a single search channel (entities, relationships, etc.).
///
/// Each channel can be independently enabled, with its own search methods,
/// reranker, and result limit. This is the Rust equivalent of Graphiti's
/// `EdgeSearchConfig`, `NodeSearchConfig`, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// Whether this channel is active in the search.
    pub enabled: bool,
    /// Which search methods to use for candidate retrieval.
    pub methods: Vec<SearchMethod>,
    /// How to rerank candidates from this channel.
    pub reranker: RerankerType,
    /// Maximum results to return from this channel.
    pub limit: usize,
    /// Minimum similarity score threshold for cosine search.
    pub sim_min_score: f32,
    /// Maximum BFS/traversal depth.
    pub max_depth: usize,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            methods: vec![SearchMethod::Cosine, SearchMethod::Bm25],
            reranker: RerankerType::Rrf,
            limit: 10,
            sim_min_score: 0.0,
            max_depth: 3,
        }
    }
}

impl ChannelConfig {
    /// Create a disabled channel config.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Create with a single search method.
    pub fn single(method: SearchMethod) -> Self {
        Self {
            methods: vec![method],
            ..Default::default()
        }
    }

    /// Set the reranker.
    pub fn with_reranker(mut self, reranker: RerankerType) -> Self {
        self.reranker = reranker;
        self
    }

    /// Set the result limit.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

// ─── Top-Level SearchConfig ───────────────────────────────────────────

/// Multi-channel search configuration.
///
/// Controls which graph element types to search, which methods to use
/// for each, and how to rerank results per channel. This is the single
/// top-level configuration passed to `HybridRetriever::search_with_config()`.
///
/// Equivalent to Graphiti's `SearchConfig` which composes:
/// - `EdgeSearchConfig` (our `relationships`)
/// - `NodeSearchConfig` (our `entities`)
/// - `EpisodeSearchConfig` (our `episodes`)
/// - `CommunitySearchConfig` (our `communities`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Entity (node) search configuration.
    pub entities: ChannelConfig,
    /// Relationship (edge) search configuration.
    pub relationships: ChannelConfig,
    /// Episode (provenance) search configuration.
    pub episodes: ChannelConfig,
    /// Community (cluster) search configuration.
    pub communities: ChannelConfig,
    /// Global minimum reranker score threshold.
    pub reranker_min_score: f32,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            entities: ChannelConfig::default(),
            relationships: ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Bm25],
                reranker: RerankerType::Rrf,
                limit: 10,
                ..Default::default()
            },
            episodes: ChannelConfig {
                enabled: false,
                methods: vec![SearchMethod::Bm25],
                reranker: RerankerType::Rrf,
                limit: 5,
                ..Default::default()
            },
            communities: ChannelConfig {
                enabled: false,
                methods: vec![SearchMethod::Cosine],
                reranker: RerankerType::Rrf,
                limit: 3,
                ..Default::default()
            },
            reranker_min_score: 0.0,
        }
    }
}

impl SearchConfig {
    /// Create a builder for constructing a SearchConfig.
    pub fn builder() -> SearchConfigBuilder {
        SearchConfigBuilder::default()
    }

    /// Quick entity-only search with cosine similarity.
    pub fn entity_only() -> Self {
        Self {
            entities: ChannelConfig::default(),
            relationships: ChannelConfig::disabled(),
            episodes: ChannelConfig::disabled(),
            communities: ChannelConfig::disabled(),
            ..Default::default()
        }
    }

    /// All channels enabled.
    pub fn all_channels() -> Self {
        Self {
            entities: ChannelConfig::default(),
            relationships: ChannelConfig::default(),
            episodes: ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Bm25],
                reranker: RerankerType::Rrf,
                limit: 5,
                ..Default::default()
            },
            communities: ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Bm25],
                reranker: RerankerType::Rrf,
                limit: 3,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Get a list of enabled channel names.
    pub fn enabled_channels(&self) -> Vec<&'static str> {
        let mut channels = Vec::new();
        if self.entities.enabled {
            channels.push("entities");
        }
        if self.relationships.enabled {
            channels.push("relationships");
        }
        if self.episodes.enabled {
            channels.push("episodes");
        }
        if self.communities.enabled {
            channels.push("communities");
        }
        channels
    }
}

// ─── Builder ──────────────────────────────────────────────────────────

/// Builder for `SearchConfig`.
#[derive(Default)]
pub struct SearchConfigBuilder {
    entities: Option<ChannelConfig>,
    relationships: Option<ChannelConfig>,
    episodes: Option<ChannelConfig>,
    communities: Option<ChannelConfig>,
    reranker_min_score: Option<f32>,
}

impl SearchConfigBuilder {
    /// Configure entity search.
    pub fn entities(mut self, cfg: ChannelConfig) -> Self {
        self.entities = Some(cfg);
        self
    }

    /// Configure relationship search.
    pub fn relationships(mut self, cfg: ChannelConfig) -> Self {
        self.relationships = Some(cfg);
        self
    }

    /// Configure episode search.
    pub fn episodes(mut self, cfg: ChannelConfig) -> Self {
        self.episodes = Some(cfg);
        self
    }

    /// Configure community search.
    pub fn communities(mut self, cfg: ChannelConfig) -> Self {
        self.communities = Some(cfg);
        self
    }

    /// Set global minimum reranker score.
    pub fn min_score(mut self, score: f32) -> Self {
        self.reranker_min_score = Some(score);
        self
    }

    /// Build the SearchConfig.
    pub fn build(self) -> SearchConfig {
        let defaults = SearchConfig::default();
        SearchConfig {
            entities: self.entities.unwrap_or(defaults.entities),
            relationships: self.relationships.unwrap_or(defaults.relationships),
            episodes: self.episodes.unwrap_or(defaults.episodes),
            communities: self.communities.unwrap_or(defaults.communities),
            reranker_min_score: self.reranker_min_score.unwrap_or(defaults.reranker_min_score),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SearchConfig::default();
        assert!(config.entities.enabled);
        assert!(config.relationships.enabled);
        assert!(!config.episodes.enabled);
        assert!(!config.communities.enabled);
    }

    #[test]
    fn test_builder() {
        let config = SearchConfig::builder()
            .entities(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine],
                reranker: RerankerType::Mmr { lambda: 0.7 },
                limit: 20,
                ..Default::default()
            })
            .episodes(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Bm25],
                ..Default::default()
            })
            .build();

        assert!(config.entities.enabled);
        assert_eq!(config.entities.limit, 20);
        assert!(config.episodes.enabled);
        assert!(matches!(config.entities.reranker, RerankerType::Mmr { lambda } if (lambda - 0.7).abs() < f32::EPSILON));
    }

    #[test]
    fn test_entity_only() {
        let config = SearchConfig::entity_only();
        assert!(config.entities.enabled);
        assert!(!config.relationships.enabled);
        assert!(!config.episodes.enabled);
        assert!(!config.communities.enabled);
    }

    #[test]
    fn test_all_channels() {
        let config = SearchConfig::all_channels();
        let channels = config.enabled_channels();
        assert_eq!(channels.len(), 4);
    }

    #[test]
    fn test_disabled_channel() {
        let cfg = ChannelConfig::disabled();
        assert!(!cfg.enabled);
    }

    #[test]
    fn test_channel_config_builder_methods() {
        let cfg = ChannelConfig::single(SearchMethod::Ppr)
            .with_reranker(RerankerType::Mmr { lambda: 0.5 })
            .with_limit(25);

        assert_eq!(cfg.methods.len(), 1);
        assert_eq!(cfg.methods[0], SearchMethod::Ppr);
        assert_eq!(cfg.limit, 25);
    }
}
