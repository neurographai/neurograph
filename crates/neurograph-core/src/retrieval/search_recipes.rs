// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Pre-built search configuration recipes.
//!
//! Analogous to Graphiti's 15+ `search_config_recipes.py` but with
//! NeuroGraph-specific capabilities (PPR, DRIFT, conversation replay).
//!
//! Each recipe is a pre-configured `SearchConfig` for common use cases.
//! Use these as starting points and customize via the builder pattern.

use super::search_config::*;

/// Pre-built search configurations matching common use cases.
///
/// Organized by:
/// - **Combined** — search across all channels
/// - **Entity-focused** — entity-only searches
/// - **Graph-aware** — leverage graph structure (PPR, BFS)
/// - **Conversation** — episode/saga replay
/// - **Community** — cluster-level analysis
pub struct SearchRecipes;

impl SearchRecipes {
    // ═══════════════════════════════════════════════════════════════════
    // Combined (all channels)
    // ═══════════════════════════════════════════════════════════════════

    /// Combined hybrid search with RRF reranking across all channels.
    ///
    /// Equivalent to Graphiti's `COMBINED_HYBRID_SEARCH_RRF`.
    pub fn combined_hybrid_rrf() -> SearchConfig {
        SearchConfig::builder()
            .entities(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Bm25],
                reranker: RerankerType::Rrf,
                limit: 10,
                ..Default::default()
            })
            .relationships(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Bm25],
                reranker: RerankerType::Rrf,
                limit: 10,
                ..Default::default()
            })
            .episodes(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Bm25],
                reranker: RerankerType::Rrf,
                limit: 5,
                ..Default::default()
            })
            .communities(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Bm25],
                reranker: RerankerType::Rrf,
                limit: 3,
                ..Default::default()
            })
            .build()
    }

    /// Combined hybrid search with MMR reranking for diverse results.
    ///
    /// Equivalent to Graphiti's `COMBINED_HYBRID_SEARCH_MMR`.
    pub fn combined_hybrid_mmr(lambda: f32) -> SearchConfig {
        SearchConfig::builder()
            .entities(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Bm25],
                reranker: RerankerType::Mmr { lambda },
                limit: 10,
                ..Default::default()
            })
            .relationships(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Bm25],
                reranker: RerankerType::Mmr { lambda },
                limit: 10,
                ..Default::default()
            })
            .episodes(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Bm25],
                reranker: RerankerType::Rrf,
                limit: 5,
                ..Default::default()
            })
            .communities(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine],
                reranker: RerankerType::Mmr { lambda },
                limit: 3,
                ..Default::default()
            })
            .build()
    }

    /// Combined hybrid search with cross-encoder reranking (highest accuracy).
    ///
    /// Equivalent to Graphiti's `COMBINED_HYBRID_SEARCH_CROSS_ENCODER`.
    pub fn combined_hybrid_cross_encoder() -> SearchConfig {
        SearchConfig::builder()
            .entities(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Bm25, SearchMethod::Bfs],
                reranker: RerankerType::CrossEncoder {
                    model: "default".to_string(),
                },
                limit: 10,
                ..Default::default()
            })
            .relationships(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Bm25, SearchMethod::Bfs],
                reranker: RerankerType::CrossEncoder {
                    model: "default".to_string(),
                },
                limit: 10,
                ..Default::default()
            })
            .episodes(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Bm25],
                reranker: RerankerType::CrossEncoder {
                    model: "default".to_string(),
                },
                limit: 5,
                ..Default::default()
            })
            .communities(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Bm25],
                reranker: RerankerType::CrossEncoder {
                    model: "default".to_string(),
                },
                limit: 3,
                ..Default::default()
            })
            .build()
    }

    // ═══════════════════════════════════════════════════════════════════
    // Entity-focused
    // ═══════════════════════════════════════════════════════════════════

    /// Entity-only hybrid search with RRF.
    ///
    /// Equivalent to Graphiti's `NODE_HYBRID_SEARCH_RRF`.
    pub fn entity_hybrid_rrf() -> SearchConfig {
        SearchConfig::builder()
            .entities(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Bm25],
                reranker: RerankerType::Rrf,
                limit: 10,
                ..Default::default()
            })
            .relationships(ChannelConfig::disabled())
            .episodes(ChannelConfig::disabled())
            .communities(ChannelConfig::disabled())
            .build()
    }

    /// Entity-only with MMR for diversity.
    ///
    /// Equivalent to Graphiti's `NODE_HYBRID_SEARCH_MMR`.
    pub fn entity_hybrid_mmr(lambda: f32) -> SearchConfig {
        SearchConfig::builder()
            .entities(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Bm25],
                reranker: RerankerType::Mmr { lambda },
                limit: 10,
                ..Default::default()
            })
            .relationships(ChannelConfig::disabled())
            .episodes(ChannelConfig::disabled())
            .communities(ChannelConfig::disabled())
            .build()
    }

    /// Entity-only with episode mentions reranking.
    ///
    /// Equivalent to Graphiti's `NODE_HYBRID_SEARCH_EPISODE_MENTIONS`.
    pub fn entity_hybrid_episode_mentions() -> SearchConfig {
        SearchConfig::builder()
            .entities(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Bm25],
                reranker: RerankerType::EpisodeMentions,
                limit: 10,
                ..Default::default()
            })
            .relationships(ChannelConfig::disabled())
            .episodes(ChannelConfig::disabled())
            .communities(ChannelConfig::disabled())
            .build()
    }

    /// Entity-only cosine search (fastest, no BM25 overhead).
    pub fn entity_only_cosine() -> SearchConfig {
        SearchConfig::builder()
            .entities(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine],
                reranker: RerankerType::None,
                limit: 20,
                ..Default::default()
            })
            .relationships(ChannelConfig::disabled())
            .episodes(ChannelConfig::disabled())
            .communities(ChannelConfig::disabled())
            .build()
    }

    // ═══════════════════════════════════════════════════════════════════
    // Relationship-focused
    // ═══════════════════════════════════════════════════════════════════

    /// Relationship-only hybrid search with RRF.
    ///
    /// Equivalent to Graphiti's `EDGE_HYBRID_SEARCH_RRF`.
    pub fn relationship_hybrid_rrf() -> SearchConfig {
        SearchConfig::builder()
            .entities(ChannelConfig::disabled())
            .relationships(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Bm25],
                reranker: RerankerType::Rrf,
                limit: 10,
                ..Default::default()
            })
            .episodes(ChannelConfig::disabled())
            .communities(ChannelConfig::disabled())
            .build()
    }

    /// Relationship-only with node distance reranking.
    ///
    /// Equivalent to Graphiti's `EDGE_HYBRID_SEARCH_NODE_DISTANCE`.
    pub fn relationship_hybrid_node_distance(center_node_id: String) -> SearchConfig {
        SearchConfig::builder()
            .entities(ChannelConfig::disabled())
            .relationships(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Bm25],
                reranker: RerankerType::NodeDistance {
                    center_node_id: Some(center_node_id),
                },
                limit: 10,
                ..Default::default()
            })
            .episodes(ChannelConfig::disabled())
            .communities(ChannelConfig::disabled())
            .build()
    }

    // ═══════════════════════════════════════════════════════════════════
    // Graph-aware (NeuroGraph unique)
    // ═══════════════════════════════════════════════════════════════════

    /// PPR-enhanced graph-aware search.
    ///
    /// Combines cosine similarity with Personalized PageRank for
    /// structurally-important entity discovery. (NeuroGraph unique)
    pub fn graph_aware_ppr() -> SearchConfig {
        SearchConfig::builder()
            .entities(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Ppr],
                reranker: RerankerType::Rrf,
                limit: 10,
                ..Default::default()
            })
            .relationships(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Bfs],
                reranker: RerankerType::Rrf,
                limit: 10,
                ..Default::default()
            })
            .episodes(ChannelConfig::disabled())
            .communities(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine],
                reranker: RerankerType::Rrf,
                limit: 3,
                ..Default::default()
            })
            .build()
    }

    /// DRIFT adaptive search — dynamic reasoning with flexible traversal.
    ///
    /// Uses DRIFT to adaptively switch between local and global search.
    /// Combined with MMR for diverse results. (NeuroGraph unique)
    pub fn drift_adaptive() -> SearchConfig {
        SearchConfig::builder()
            .entities(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::DriftAdaptive],
                reranker: RerankerType::Mmr { lambda: 0.7 },
                limit: 15,
                ..Default::default()
            })
            .relationships(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::DriftAdaptive],
                reranker: RerankerType::Mmr { lambda: 0.7 },
                limit: 10,
                ..Default::default()
            })
            .episodes(ChannelConfig::disabled())
            .communities(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine],
                reranker: RerankerType::Rrf,
                limit: 5,
                ..Default::default()
            })
            .build()
    }

    /// Neighborhood search — BFS from a center node.
    ///
    /// Explores the local neighborhood of a specific entity,
    /// reranked by graph distance. (NeuroGraph unique)
    pub fn neighborhood_search(center_node_id: String) -> SearchConfig {
        SearchConfig::builder()
            .entities(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Bfs],
                reranker: RerankerType::NodeDistance {
                    center_node_id: Some(center_node_id.clone()),
                },
                limit: 20,
                ..Default::default()
            })
            .relationships(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Bfs],
                reranker: RerankerType::NodeDistance {
                    center_node_id: Some(center_node_id),
                },
                limit: 20,
                ..Default::default()
            })
            .episodes(ChannelConfig::disabled())
            .communities(ChannelConfig::disabled())
            .build()
    }

    // ═══════════════════════════════════════════════════════════════════
    // Conversation / Episode replay
    // ═══════════════════════════════════════════════════════════════════

    /// Conversation replay — episode-focused search.
    ///
    /// Designed for "What was discussed in conversation X?" queries.
    /// Returns episodes in relevance order.
    pub fn conversation_replay() -> SearchConfig {
        SearchConfig::builder()
            .entities(ChannelConfig::disabled())
            .relationships(ChannelConfig::disabled())
            .episodes(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Bm25, SearchMethod::Cosine],
                reranker: RerankerType::Rrf,
                limit: 50,
                ..Default::default()
            })
            .communities(ChannelConfig::disabled())
            .build()
    }

    // ═══════════════════════════════════════════════════════════════════
    // Community-focused
    // ═══════════════════════════════════════════════════════════════════

    /// Community-only search for thematic summarization.
    ///
    /// Equivalent to Graphiti's `COMMUNITY_HYBRID_SEARCH_RRF`.
    pub fn community_hybrid_rrf() -> SearchConfig {
        SearchConfig::builder()
            .entities(ChannelConfig::disabled())
            .relationships(ChannelConfig::disabled())
            .episodes(ChannelConfig::disabled())
            .communities(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Bm25],
                reranker: RerankerType::Rrf,
                limit: 10,
                ..Default::default()
            })
            .build()
    }

    /// Community search with MMR for diverse cluster summaries.
    ///
    /// Equivalent to Graphiti's `COMMUNITY_HYBRID_SEARCH_MMR`.
    pub fn community_hybrid_mmr(lambda: f32) -> SearchConfig {
        SearchConfig::builder()
            .entities(ChannelConfig::disabled())
            .relationships(ChannelConfig::disabled())
            .episodes(ChannelConfig::disabled())
            .communities(ChannelConfig {
                enabled: true,
                methods: vec![SearchMethod::Cosine, SearchMethod::Bm25],
                reranker: RerankerType::Mmr { lambda },
                limit: 10,
                ..Default::default()
            })
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combined_hybrid_rrf() {
        let config = SearchRecipes::combined_hybrid_rrf();
        assert!(config.entities.enabled);
        assert!(config.relationships.enabled);
        assert!(config.episodes.enabled);
        assert!(config.communities.enabled);
        assert!(matches!(config.entities.reranker, RerankerType::Rrf));
    }

    #[test]
    fn test_combined_hybrid_mmr() {
        let config = SearchRecipes::combined_hybrid_mmr(0.7);
        assert!(matches!(
            config.entities.reranker,
            RerankerType::Mmr { lambda } if (lambda - 0.7).abs() < f32::EPSILON
        ));
    }

    #[test]
    fn test_entity_only() {
        let config = SearchRecipes::entity_only_cosine();
        assert!(config.entities.enabled);
        assert!(!config.relationships.enabled);
        assert!(!config.episodes.enabled);
        assert!(!config.communities.enabled);
        assert_eq!(config.entities.methods.len(), 1);
    }

    #[test]
    fn test_conversation_replay() {
        let config = SearchRecipes::conversation_replay();
        assert!(config.episodes.enabled);
        assert!(!config.entities.enabled);
        assert_eq!(config.episodes.limit, 50);
    }

    #[test]
    fn test_neighborhood_search() {
        let config = SearchRecipes::neighborhood_search("node_42".into());
        assert!(config.entities.enabled);
        assert!(matches!(
            &config.entities.reranker,
            RerankerType::NodeDistance { center_node_id: Some(id) } if id == "node_42"
        ));
    }

    #[test]
    fn test_drift_adaptive() {
        let config = SearchRecipes::drift_adaptive();
        assert!(config.entities.enabled);
        assert_eq!(config.entities.methods[0], SearchMethod::DriftAdaptive);
    }

    #[test]
    fn test_all_recipes_compile() {
        // Ensure all recipes can be constructed without panics
        let _ = SearchRecipes::combined_hybrid_rrf();
        let _ = SearchRecipes::combined_hybrid_mmr(0.5);
        let _ = SearchRecipes::combined_hybrid_cross_encoder();
        let _ = SearchRecipes::entity_hybrid_rrf();
        let _ = SearchRecipes::entity_hybrid_mmr(0.7);
        let _ = SearchRecipes::entity_hybrid_episode_mentions();
        let _ = SearchRecipes::entity_only_cosine();
        let _ = SearchRecipes::relationship_hybrid_rrf();
        let _ = SearchRecipes::relationship_hybrid_node_distance("x".into());
        let _ = SearchRecipes::graph_aware_ppr();
        let _ = SearchRecipes::drift_adaptive();
        let _ = SearchRecipes::neighborhood_search("y".into());
        let _ = SearchRecipes::conversation_replay();
        let _ = SearchRecipes::community_hybrid_rrf();
        let _ = SearchRecipes::community_hybrid_mmr(0.5);
    }
}
