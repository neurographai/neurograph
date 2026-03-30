// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Database driver abstraction layer.
//!
//! Influenced by Graphiti's `GraphDriver` (driver.py L90-211):
//! - Abstract trait with GraphProvider enum
//! - `execute_query()`, `session()`, `build_indices_and_constraints()`
//! - Multiple driver implementations (Neo4j, FalkorDB, Kuzu)

pub mod embedded;
pub mod memory;
pub mod traits;

pub use traits::{GraphDriver, GraphDriverConfig, Subgraph};
