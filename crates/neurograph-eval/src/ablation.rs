// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Ablation study runner for the NeuroGraph research paper.
//!
//! Defines 9 configurations that systematically toggle architectural
//! components to measure their individual and combined contributions.
//!
//! # Ablation Matrix
//!
//! | # | Config Name                | Hybrid | Tiered | MAGMA | RL-Forget |
//! |---|----------------------------|--------|--------|-------|-----------|
//! | 1 | Cosine-Only Baseline       |   ✗    |   ✗    |   ✗   |     ✗     |
//! | 2 | + BM25 Hybrid (RRF)        |   ✓    |   ✗    |   ✗   |     ✗     |
//! | 3 | + Cross-Encoder Rerank     |   ✓+   |   ✗    |   ✗   |     ✗     |
//! | 4 | + Tiered Memory            |   ✓    |   ✓    |   ✗   |     ✗     |
//! | 5 | + MAGMA Multi-Graph        |   ✓    |   ✗    |   ✓   |     ✗     |
//! | 6 | + RL Forgetting            |   ✓    |   ✗    |   ✗   |     ✓     |
//! | 7 | Tiered + MAGMA             |   ✓    |   ✓    |   ✓   |     ✗     |
//! | 8 | Tiered + MAGMA + RL        |   ✓    |   ✓    |   ✓   |     ✓     |
//! | 9 | Full System                |   ✓+   |   ✓    |   ✓   |     ✓     |
//!
//! Results are exported as JSONL, JSON, CSV, and LaTeX tables.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A single ablation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AblationConfig {
    /// Short identifier (e.g., "cosine_only").
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Whether to use hybrid retrieval (BM25 + vector).
    pub hybrid_retrieval: bool,
    /// Whether to use cross-encoder reranking.
    pub cross_encoder_rerank: bool,
    /// Whether to enable tiered memory (L1–L4).
    pub tiered_memory: bool,
    /// Whether to enable MAGMA multi-graph.
    pub multigraph: bool,
    /// Whether to enable RL-guided forgetting.
    pub rl_forgetting: bool,
}

impl AblationConfig {
    /// Get the full ablation matrix (9 configurations).
    pub fn full_matrix() -> Vec<AblationConfig> {
        vec![
            // 1. Cosine-Only Baseline
            AblationConfig {
                id: "cosine_only".into(),
                name: "Cosine-Only Baseline".into(),
                hybrid_retrieval: false,
                cross_encoder_rerank: false,
                tiered_memory: false,
                multigraph: false,
                rl_forgetting: false,
            },
            // 2. + BM25 Hybrid (RRF)
            AblationConfig {
                id: "hybrid_rrf".into(),
                name: "+ BM25 Hybrid (RRF)".into(),
                hybrid_retrieval: true,
                cross_encoder_rerank: false,
                tiered_memory: false,
                multigraph: false,
                rl_forgetting: false,
            },
            // 3. + Cross-Encoder Rerank
            AblationConfig {
                id: "hybrid_rerank".into(),
                name: "+ Cross-Encoder Rerank".into(),
                hybrid_retrieval: true,
                cross_encoder_rerank: true,
                tiered_memory: false,
                multigraph: false,
                rl_forgetting: false,
            },
            // 4. + Tiered Memory
            AblationConfig {
                id: "tiered".into(),
                name: "+ Tiered Memory (L1–L4)".into(),
                hybrid_retrieval: true,
                cross_encoder_rerank: false,
                tiered_memory: true,
                multigraph: false,
                rl_forgetting: false,
            },
            // 5. + MAGMA Multi-Graph
            AblationConfig {
                id: "magma".into(),
                name: "+ MAGMA Multi-Graph".into(),
                hybrid_retrieval: true,
                cross_encoder_rerank: false,
                tiered_memory: false,
                multigraph: true,
                rl_forgetting: false,
            },
            // 6. + RL Forgetting
            AblationConfig {
                id: "rl_forget".into(),
                name: "+ RL-Guided Forgetting".into(),
                hybrid_retrieval: true,
                cross_encoder_rerank: false,
                tiered_memory: false,
                multigraph: false,
                rl_forgetting: true,
            },
            // 7. Tiered + MAGMA
            AblationConfig {
                id: "tiered_magma".into(),
                name: "Tiered + MAGMA".into(),
                hybrid_retrieval: true,
                cross_encoder_rerank: false,
                tiered_memory: true,
                multigraph: true,
                rl_forgetting: false,
            },
            // 8. Tiered + MAGMA + RL
            AblationConfig {
                id: "tiered_magma_rl".into(),
                name: "Tiered + MAGMA + RL".into(),
                hybrid_retrieval: true,
                cross_encoder_rerank: false,
                tiered_memory: true,
                multigraph: true,
                rl_forgetting: true,
            },
            // 9. Full System
            AblationConfig {
                id: "full_system".into(),
                name: "Full System".into(),
                hybrid_retrieval: true,
                cross_encoder_rerank: true,
                tiered_memory: true,
                multigraph: true,
                rl_forgetting: true,
            },
        ]
    }
}

/// Result of a single ablation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AblationResult {
    /// Configuration used for this run.
    pub config: AblationConfig,
    /// Overall accuracy.
    pub accuracy: f64,
    /// Per-category accuracy breakdown.
    pub by_category: HashMap<String, f64>,
    /// Mean latency in ms.
    pub mean_latency_ms: f64,
    /// Total instances evaluated.
    pub total_instances: usize,
}

impl AblationResult {
    /// Build from an eval report.
    pub fn from_report(config: AblationConfig, report: &crate::longmemeval::EvalReport) -> Self {
        let by_category: HashMap<String, f64> = report
            .by_category
            .iter()
            .map(|(qt, cs)| (qt.to_string(), cs.accuracy))
            .collect();

        Self {
            config,
            accuracy: report.overall_accuracy,
            by_category,
            mean_latency_ms: report.mean_latency_ms,
            total_instances: report.total_instances,
        }
    }
}

/// Collection of all ablation results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AblationSuite {
    /// Timestamp of the run.
    pub timestamp: String,
    /// Model used.
    pub model: String,
    /// Dataset used.
    pub dataset: String,
    /// Individual run results.
    pub results: Vec<AblationResult>,
}

impl AblationSuite {
    /// Create a new empty suite.
    pub fn new(model: &str, dataset: &str) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            model: model.into(),
            dataset: dataset.into(),
            results: Vec::new(),
        }
    }

    /// Print a summary table.
    pub fn print_summary(&self) {
        let bar = "═".repeat(100);
        let thin = "─".repeat(100);

        println!("\n{bar}");
        println!("  NeuroGraph Ablation Study Results");
        println!("  Model: {}  |  Dataset: {}  |  {}", self.model, self.dataset, self.timestamp);
        println!("{bar}");
        println!(
            "  {:<4} {:<30} {:>8} {:>8} {:>8} {:>8} {:>8} {:>10}",
            "#", "Configuration", "Acc.", "IE", "MSR", "TR", "KU", "Lat.(ms)"
        );
        println!("{thin}");

        for (i, r) in self.results.iter().enumerate() {
            let ie = r.by_category.get("Information Extraction").copied().unwrap_or(0.0);
            let msr = r.by_category.get("Multi-Session Reasoning").copied().unwrap_or(0.0);
            let tr = r.by_category.get("Temporal Reasoning").copied().unwrap_or(0.0);
            let ku = r.by_category.get("Knowledge Update").copied().unwrap_or(0.0);

            println!(
                "  {:<4} {:<30} {:>7.1}% {:>7.1}% {:>7.1}% {:>7.1}% {:>7.1}% {:>10.0}",
                i + 1,
                r.config.name,
                r.accuracy * 100.0,
                ie * 100.0,
                msr * 100.0,
                tr * 100.0,
                ku * 100.0,
                r.mean_latency_ms,
            );
        }
        println!("{bar}\n");
    }

    /// Export results as CSV.
    pub fn export_csv(&self, path: &Path) -> anyhow::Result<()> {
        let mut wtr = csv::Writer::from_path(path)?;

        wtr.write_record([
            "config_id", "config_name", "accuracy",
            "ie_accuracy", "msr_accuracy", "tr_accuracy", "ku_accuracy", "abstention_accuracy",
            "mean_latency_ms", "total_instances",
            "hybrid", "cross_encoder", "tiered", "magma", "rl_forget",
        ])?;

        for r in &self.results {
            let ie = r.by_category.get("Information Extraction").copied().unwrap_or(0.0);
            let msr = r.by_category.get("Multi-Session Reasoning").copied().unwrap_or(0.0);
            let tr = r.by_category.get("Temporal Reasoning").copied().unwrap_or(0.0);
            let ku = r.by_category.get("Knowledge Update").copied().unwrap_or(0.0);
            let ab = r.by_category.get("Abstention").copied().unwrap_or(0.0);

            wtr.write_record(&[
                &r.config.id,
                &r.config.name,
                &format!("{:.4}", r.accuracy),
                &format!("{:.4}", ie),
                &format!("{:.4}", msr),
                &format!("{:.4}", tr),
                &format!("{:.4}", ku),
                &format!("{:.4}", ab),
                &format!("{:.1}", r.mean_latency_ms),
                &r.total_instances.to_string(),
                &r.config.hybrid_retrieval.to_string(),
                &r.config.cross_encoder_rerank.to_string(),
                &r.config.tiered_memory.to_string(),
                &r.config.multigraph.to_string(),
                &r.config.rl_forgetting.to_string(),
            ])?;
        }

        wtr.flush()?;
        Ok(())
    }

    /// Export results as JSON.
    pub fn export_json(&self, path: &Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Export results as JSONL (one line per config).
    pub fn export_jsonl(&self, path: &Path) -> anyhow::Result<()> {
        let mut content = String::new();
        for r in &self.results {
            content.push_str(&serde_json::to_string(r)?);
            content.push('\n');
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Generate a LaTeX table for the research paper.
    pub fn to_latex(&self) -> String {
        let mut latex = String::new();

        latex.push_str("\\begin{table*}[t]\n");
        latex.push_str("\\centering\n");
        latex.push_str("\\caption{Ablation study results on LongMemEval. ");
        latex.push_str("IE = Information Extraction, MSR = Multi-Session Reasoning, ");
        latex.push_str("TR = Temporal Reasoning, KU = Knowledge Update, AB = Abstention.}\n");
        latex.push_str("\\label{tab:ablation}\n");
        latex.push_str("\\begin{tabular}{ll cccccc r}\n");
        latex.push_str("\\toprule\n");
        latex.push_str("\\# & Configuration & Overall & IE & MSR & TR & KU & AB & Lat.(ms) \\\\\n");
        latex.push_str("\\midrule\n");

        for (i, r) in self.results.iter().enumerate() {
            let ie = r.by_category.get("Information Extraction").copied().unwrap_or(0.0);
            let msr = r.by_category.get("Multi-Session Reasoning").copied().unwrap_or(0.0);
            let tr = r.by_category.get("Temporal Reasoning").copied().unwrap_or(0.0);
            let ku = r.by_category.get("Knowledge Update").copied().unwrap_or(0.0);
            let ab = r.by_category.get("Abstention").copied().unwrap_or(0.0);

            // Bold the best result
            let is_best = i == self.results.len() - 1;
            let fmt = |v: f64| {
                if is_best {
                    format!("\\textbf{{{:.1}}}", v * 100.0)
                } else {
                    format!("{:.1}", v * 100.0)
                }
            };

            latex.push_str(&format!(
                "{} & {} & {} & {} & {} & {} & {} & {} & {:.0} \\\\\n",
                i + 1,
                r.config.name.replace("&", "\\&"),
                fmt(r.accuracy),
                fmt(ie),
                fmt(msr),
                fmt(tr),
                fmt(ku),
                fmt(ab),
                r.mean_latency_ms,
            ));

            // Add midrule separators
            if i == 0 || i == 2 || i == 5 {
                latex.push_str("\\midrule\n");
            }
        }

        latex.push_str("\\bottomrule\n");
        latex.push_str("\\end{tabular}\n");
        latex.push_str("\\end{table*}\n");

        latex
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_matrix_size() {
        let matrix = AblationConfig::full_matrix();
        assert_eq!(matrix.len(), 9);
    }

    #[test]
    fn test_full_matrix_first_is_baseline() {
        let matrix = AblationConfig::full_matrix();
        let baseline = &matrix[0];
        assert!(!baseline.hybrid_retrieval);
        assert!(!baseline.cross_encoder_rerank);
        assert!(!baseline.tiered_memory);
        assert!(!baseline.multigraph);
        assert!(!baseline.rl_forgetting);
    }

    #[test]
    fn test_full_matrix_last_is_full() {
        let matrix = AblationConfig::full_matrix();
        let full = &matrix[8];
        assert!(full.hybrid_retrieval);
        assert!(full.cross_encoder_rerank);
        assert!(full.tiered_memory);
        assert!(full.multigraph);
        assert!(full.rl_forgetting);
    }

    #[test]
    fn test_latex_generation() {
        let mut suite = AblationSuite::new("test-model", "test-dataset");
        suite.results.push(AblationResult {
            config: AblationConfig::full_matrix()[0].clone(),
            accuracy: 0.5,
            by_category: HashMap::new(),
            mean_latency_ms: 100.0,
            total_instances: 10,
        });

        let latex = suite.to_latex();
        assert!(latex.contains("\\begin{table*}"));
        assert!(latex.contains("Cosine-Only Baseline"));
        assert!(latex.contains("\\end{table*}"));
    }

    #[test]
    fn test_csv_export_roundtrip() {
        let mut suite = AblationSuite::new("test-model", "test-dataset");
        suite.results.push(AblationResult {
            config: AblationConfig::full_matrix()[0].clone(),
            accuracy: 0.5,
            by_category: HashMap::from([
                ("Information Extraction".into(), 0.6),
            ]),
            mean_latency_ms: 100.0,
            total_instances: 10,
        });

        let dir = tempfile::tempdir().unwrap();
        let csv_path = dir.path().join("test.csv");
        suite.export_csv(&csv_path).unwrap();

        let content = std::fs::read_to_string(&csv_path).unwrap();
        assert!(content.contains("cosine_only"));
        assert!(content.contains("0.5000"));
    }
}
