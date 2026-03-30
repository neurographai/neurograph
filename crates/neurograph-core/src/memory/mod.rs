// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Tiered memory system: L1–L4 hierarchy with automatic promotion/demotion.

pub mod tiered;
pub mod evolution;

pub use tiered::{TieredMemory, ConsolidationConfig, PromotionPolicy, TieredStats, Episode, LearnedRule, QueryContext};
pub use evolution::{MemoryEvolution, DecayPolicy, RetentionPolicy, EvolutionResult};
