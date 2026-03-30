// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Community detection algorithms and management.
//!
//! Provides:
//! - Louvain algorithm for modularity-based community detection
//! - Leiden algorithm for higher-quality community detection (refinement phase)
//! - Community summarization with LLM
//! - Incremental community updates on entity changes
//!
//! Influenced by GraphRAG's hierarchical Leiden (hierarchical_leiden.py)
//! and adapted for incremental updates (unlike GraphRAG's batch-only approach).

pub mod incremental;
pub mod leiden;
pub mod louvain;
pub mod summarizer;

pub use incremental::{IncrementalCommunityUpdater, IncrementalError, IncrementalUpdateResult};
pub use leiden::{LeidenConfig, LeidenDetector};
pub use louvain::{CommunityDetectionResult, CommunityError, LouvainConfig, LouvainDetector};
pub use summarizer::{CommunitySummarizer, CommunitySummaryResult, SummarizerError};
