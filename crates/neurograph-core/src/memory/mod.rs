// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Memory systems: tiered hierarchy, self-evolution, and self-organizing Zettelkasten.

pub mod evolution;
pub mod tiered;
pub mod zettelkasten;

pub use evolution::{DecayPolicy, EvolutionResult, MemoryEvolution, RetentionPolicy};
pub use tiered::{
    ConsolidationConfig, Episode, LearnedRule, PromotionPolicy, QueryContext, TieredMemory,
    TieredStats,
};
pub use zettelkasten::{
    AddNoteResult, ForgetResult, MemoryNote, ZettelkastenConfig, ZettelkastenMemory,
};
