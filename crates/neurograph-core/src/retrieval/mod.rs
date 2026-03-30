// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Advanced hybrid retrieval engine for NeuroGraph.
//!
//! Combines five retrieval strategies:
//! 1. **Semantic** — Vector similarity search (cosine on embeddings)
//! 2. **BM25** — Full inverted index with TF-IDF scoring
//! 3. **PPR** — Personalized PageRank from query-relevant seeds
//! 4. **Traversal** — BFS graph walk from seed entities
//! 5. **Cross-Encoder** — Reranking of fused candidates for precision
//!
//! Strategies are fused using Reciprocal Rank Fusion (RRF) and
//! optionally routed via DRIFT (Dynamic Reasoning and Inference
//! with Flexible Traversal) for adaptive local/global switching.
//!
//! Influenced by Graphiti's hybrid search, Microsoft GraphRAG's DRIFT,
//! and 2026 research on cross-encoder reranking.

pub mod bm25;
pub mod cross_encoder;
pub mod drift;
pub mod hybrid;
pub mod keyword;
pub mod ppr;
pub mod recipes;
pub mod reranker;
pub mod semantic;
pub mod traversal;

pub use hybrid::{HybridRetriever, HybridSearchResult, RetrievalWeights};
pub use bm25::BM25Index;
pub use ppr::PersonalizedPageRank;
pub use cross_encoder::{CrossEncoderReranker, RerankCandidate, RerankResult};
pub use drift::{DriftSearch, DriftResult, DriftStrategy, CommunitySummary};
