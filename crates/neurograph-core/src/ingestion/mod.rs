// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Data ingestion pipeline for NeuroGraph.
//!
//! The pipeline flow:
//! ```text
//! Source Data → Extract → Deduplicate → Resolve Conflicts → Store → Update Communities
//! ```
//!
//! Influenced by:
//! - **Graphiti** (`graphiti.py:add_episode()`): Incremental ingestion with deduplication
//! - **GraphRAG** (`extract_entities.py`): Structured LLM extraction with typed output
//! - **Cognee** (`cognee.cognify()`): Clean pipeline abstraction

pub mod conflict;
pub mod deduplication;
pub mod extractors;
pub mod pipeline;
pub mod validators;
