// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Tiered memory system: L1–L4 hierarchy with automatic promotion/demotion.

pub mod evolution;
pub mod tiered;

pub use evolution::{DecayPolicy, EvolutionResult, MemoryEvolution, RetentionPolicy};
pub use tiered::{
    ConsolidationConfig, Episode, LearnedRule, PromotionPolicy, QueryContext, TieredMemory,
    TieredStats,
};
