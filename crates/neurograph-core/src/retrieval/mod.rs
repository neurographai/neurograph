// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Hybrid retrieval engine for NeuroGraph.
//!
//! Combines three retrieval methods:
//! 1. **Semantic** — Vector similarity search
//! 2. **Keyword** — BM25-style text matching
//! 3. **Traversal** — Graph walk from seed nodes
//!
//! Results are fused using Reciprocal Rank Fusion (RRF).
//!
//! Influenced by Graphiti's hybrid search (search/search_utils.py).

pub mod hybrid;
pub mod keyword;
pub mod recipes;
pub mod reranker;
pub mod semantic;
pub mod traversal;

pub use hybrid::{HybridRetriever, HybridSearchResult, RetrievalWeights};
