// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! # NeuroGraph Evaluation Suite
//!
//! Benchmarking harness for evaluating NeuroGraph's long-term memory abilities
//! against established benchmarks and custom temporal graph metrics.
//!
//! ## Benchmarks
//!
//! - **LongMemEval**: 500 curated questions testing 5 core memory abilities:
//!   information extraction, multi-session reasoning, temporal reasoning,
//!   knowledge updates, and abstention.
//!
//! - **NeuroGraphBench** (future): Custom benchmarks testing temporal branching,
//!   multi-agent provenance, conflict resolution precision, and time-travel accuracy.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use neurograph_eval::longmemeval::{LongMemEvalRunner, LongMemEvalConfig};
//! use neurograph_core::NeuroGraph;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let ng = Arc::new(NeuroGraph::builder().build().await?);
//!     let runner = LongMemEvalRunner::new(ng, LongMemEvalConfig::default());
//!     let report = runner.evaluate_from_file("data/longmemeval_s.json").await?;
//!     println!("Overall accuracy: {:.1}%", report.overall_accuracy * 100.0);
//!     Ok(())
//! }
//! ```

pub mod longmemeval;

// Re-export key types
pub use longmemeval::{
    EvalReport, EvalResult, LongMemEvalConfig, LongMemEvalInstance, LongMemEvalRunner,
    QuestionType,
};
