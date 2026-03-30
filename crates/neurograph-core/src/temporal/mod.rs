// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Temporal engine for bi-temporal knowledge graph management.
//!
//! Provides:
//! - Point-in-time snapshots (`snapshot_at`)
//! - Fact version chains and entity history
//! - Intelligent decay and forgetting
//! - Hybrid logical clock for strict event ordering
//!
//! Influenced by Graphiti's bi-temporal model (valid_at/invalid_at)
//! and enhanced with playback scrubbing for the G6 frontend.

pub mod clock;
pub mod forgetting;
pub mod manager;
pub mod versioning;

pub use clock::LogicalClock;
pub use forgetting::{ForgettingConfig, ForgettingEngine, ForgettingError};
pub use manager::{TemporalDiff, TemporalError, TemporalManager, TemporalSnapshot};
pub use versioning::{
    EntityChangeType, EntityHistory, EntityHistoryEntry, FactVersion, FactVersionChain,
};
