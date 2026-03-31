// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Advanced hybrid retrieval engine for NeuroGraph.
//!
//! Combines seven retrieval strategies:
//! 1. **Semantic** — Vector similarity search (cosine on embeddings)
//! 2. **BM25** — Full inverted index with TF-IDF scoring
//! 3. **PPR** — Personalized PageRank from query-relevant seeds
//! 4. **Traversal** — BFS graph walk from seed entities
//! 5. **Cross-Encoder** — Reranking of fused candidates for precision
//! 6. **MMR** — Maximal Marginal Relevance for diversity-aware reranking
//! 7. **Node Distance** — Graph proximity reranking from focal nodes
//!
//! Strategies are fused using Reciprocal Rank Fusion (RRF), MMR, or
//! cross-encoder reranking. Optionally routed via DRIFT (Dynamic
//! Reasoning and Inference with Flexible Traversal) for adaptive
//! local/global switching.
//!
//! The composable `SearchConfig` system allows per-channel (entity,
//! relationship, episode, community) search method and reranker
//! selection via 15+ pre-built `SearchRecipes`.
//!
//! Influenced by Graphiti's hybrid search, Microsoft GraphRAG's DRIFT,
//! and 2026 research on cross-encoder reranking.

pub mod bm25;
pub mod cross_encoder;
pub mod drift;
pub mod episode_mentions;
pub mod hybrid;
pub mod keyword;
pub mod mmr;
pub mod node_distance;
pub mod ppr;
pub mod recipes;
pub mod reranker;
pub mod search_config;
pub mod search_recipes;
pub mod search_results;
pub mod semantic;
pub mod traversal;

// Legacy re-exports (backward compatibility)
pub use bm25::BM25Index;
pub use cross_encoder::{CrossEncoderReranker, RerankCandidate, RerankResult};
pub use drift::{CommunitySummary, DriftResult, DriftSearch, DriftStrategy};
pub use hybrid::{HybridRetriever, HybridSearchResult, RetrievalWeights};
pub use ppr::PersonalizedPageRank;

// New re-exports (P0 gap closures)
pub use mmr::{MmrReranker, SimilarityMetric};
pub use node_distance::NodeDistanceReranker;
pub use search_config::{ChannelConfig, RerankerType, SearchConfig, SearchMethod};
pub use search_results::{Channel, FlatResult, SearchMetadata, SearchResults, ScoredItem};
pub use episode_mentions::{EpisodeMentionsReranker, HasEpisodeIds};
