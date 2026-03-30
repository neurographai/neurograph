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

pub mod louvain;
pub mod leiden;
pub mod summarizer;
pub mod incremental;

pub use louvain::{LouvainDetector, LouvainConfig, CommunityDetectionResult, CommunityError};
pub use leiden::{LeidenDetector, LeidenConfig};
pub use summarizer::{CommunitySummarizer, CommunitySummaryResult, SummarizerError};
pub use incremental::{IncrementalCommunityUpdater, IncrementalUpdateResult, IncrementalError};

