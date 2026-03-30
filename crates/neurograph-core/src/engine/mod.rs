// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Query engine with adaptive routing and cost-aware strategy selection.
//!
//! Influenced by:
//! - GraphRAG's structured_search (local/global/DRIFT modes)
//! - Graphiti's hybrid retrieval engine

pub mod budget;
pub mod context;
pub mod router;
pub mod strategies;
