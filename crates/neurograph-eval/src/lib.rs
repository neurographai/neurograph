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
//! - **Ablation Study**: 9-configuration matrix systematically toggling
//!   hybrid retrieval, cross-encoder reranking, tiered memory, MAGMA
//!   multi-graph, and RL-guided forgetting. Exports to CSV, JSON, JSONL,
//!   and LaTeX tables.
//!
//! ## CLI
//!
//! ```bash
//! # Run LongMemEval benchmark
//! neurograph-eval long-mem-eval --dataset data/longmemeval_s.json
//!
//! # Run full ablation study
//! neurograph-eval ablation --dataset data/longmemeval_s.json --output results/
//!
//! # Generate LaTeX tables from results
//! neurograph-eval tables --input results/ablation.json --format latex
//!
//! # List all 19 supported embedding models
//! neurograph-eval list-models
//! ```
//!
//! ## Programmatic Usage
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
pub mod ablation;

// Re-export key types
pub use longmemeval::{
    EvalReport, EvalResult, LongMemEvalConfig, LongMemEvalInstance, LongMemEvalRunner,
    QuestionType,
};

pub use ablation::{AblationConfig, AblationResult, AblationSuite};
