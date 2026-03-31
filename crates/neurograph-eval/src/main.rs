// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! NeuroGraph Evaluation CLI
//!
//! # Commands
//!
//! ```bash
//! # Run LongMemEval benchmark
//! neurograph-eval long-mem-eval --dataset data/longmemeval_s.json
//!
//! # Run full ablation study
//! neurograph-eval ablation --dataset data/longmemeval_s.json --output results/
//!
//! # Generate LaTeX tables from existing results
//! neurograph-eval tables --input results/ablation.json --format latex
//! ```

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;

/// NeuroGraph Evaluation Suite — LongMemEval & Ablation Runner
#[derive(Parser)]
#[command(name = "neurograph-eval")]
#[command(version, about = "Benchmark NeuroGraph's long-term memory abilities")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the LongMemEval benchmark
    LongMemEval {
        /// Path to the LongMemEval dataset JSON
        #[arg(short, long)]
        dataset: PathBuf,

        /// Maximum instances to evaluate (default: all)
        #[arg(short, long)]
        max_instances: Option<usize>,

        /// Log progress every N instances
        #[arg(long, default_value = "50")]
        progress_interval: usize,

        /// Whether to reset graph between instances
        #[arg(long, default_value = "true")]
        reset: bool,

        /// Output directory for results
        #[arg(short, long, default_value = "results")]
        output: PathBuf,
    },

    /// Run the full ablation study (9 configurations)
    Ablation {
        /// Path to the LongMemEval dataset JSON
        #[arg(short, long)]
        dataset: PathBuf,

        /// Maximum instances per ablation config
        #[arg(short, long)]
        max_instances: Option<usize>,

        /// Output directory for results
        #[arg(short, long, default_value = "results")]
        output: PathBuf,

        /// Only run configs matching this ID prefix
        #[arg(long)]
        filter: Option<String>,
    },

    /// Generate tables from existing ablation results
    Tables {
        /// Path to ablation results JSON
        #[arg(short, long)]
        input: PathBuf,

        /// Output format: latex, csv, json, jsonl
        #[arg(short, long, default_value = "latex")]
        format: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// List all available embedding models in the registry
    ListModels,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter)),
        )
        .init();

    match cli.command {
        Commands::LongMemEval {
            dataset,
            max_instances,
            progress_interval,
            reset,
            output,
        } => {
            run_longmemeval(dataset, max_instances, progress_interval, reset, output).await?;
        }
        Commands::Ablation {
            dataset,
            max_instances,
            output,
            filter,
        } => {
            run_ablation(dataset, max_instances, output, filter).await?;
        }
        Commands::Tables {
            input, format, output,
        } => {
            run_tables(input, format, output)?;
        }
        Commands::ListModels => {
            list_models();
        }
    }

    Ok(())
}

/// Run a single LongMemEval benchmark.
async fn run_longmemeval(
    dataset: PathBuf,
    max_instances: Option<usize>,
    progress_interval: usize,
    reset: bool,
    output: PathBuf,
) -> anyhow::Result<()> {
    use neurograph_eval::longmemeval::{LongMemEvalConfig, LongMemEvalRunner};

    println!("\n🧠 NeuroGraph LongMemEval Evaluation");
    println!("   Dataset: {}", dataset.display());
    println!("   Max instances: {}", max_instances.map_or("all".into(), |n| n.to_string()));
    println!();

    let ng = Arc::new(neurograph_core::NeuroGraph::builder().build().await?);

    let config = LongMemEvalConfig {
        use_llm_judge: false,
        progress_interval,
        max_instances,
        reset_between_instances: reset,
    };

    let runner = LongMemEvalRunner::new(ng, config);
    let report = runner.evaluate_from_file(dataset.to_str().unwrap()).await
        .map_err(|e| anyhow::anyhow!("Evaluation failed: {}", e))?;

    report.print_summary();

    // Export results
    std::fs::create_dir_all(&output)?;

    let json_path = output.join("longmemeval_results.json");
    let json = serde_json::to_string_pretty(&report)?;
    std::fs::write(&json_path, json)?;
    println!("📊 Results saved to {}", json_path.display());

    let csv_path = output.join("longmemeval_results.csv");
    export_report_csv(&report, &csv_path)?;
    println!("📊 CSV saved to {}", csv_path.display());

    Ok(())
}

/// Run the full ablation study.
async fn run_ablation(
    dataset: PathBuf,
    max_instances: Option<usize>,
    output: PathBuf,
    filter: Option<String>,
) -> anyhow::Result<()> {
    use neurograph_eval::ablation::{AblationConfig, AblationResult, AblationSuite};
    use neurograph_eval::longmemeval::{LongMemEvalConfig, LongMemEvalRunner};

    let configs = AblationConfig::full_matrix();
    let configs: Vec<_> = if let Some(ref prefix) = filter {
        configs.into_iter().filter(|c| c.id.starts_with(prefix)).collect()
    } else {
        configs
    };

    println!("\n🧪 NeuroGraph Ablation Study");
    println!("   Dataset: {}", dataset.display());
    println!("   Configurations: {}", configs.len());
    println!("   Max instances per config: {}", max_instances.map_or("all".into(), |n| n.to_string()));
    println!();

    let bar = indicatif::ProgressBar::new(configs.len() as u64);
    bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("   {bar:40.cyan/blue} {pos}/{len} configs [{elapsed_precise}]")
            .unwrap()
    );

    let mut suite = AblationSuite::new("hash-embedder-v1", dataset.to_str().unwrap_or("unknown"));

    for config in &configs {
        bar.set_message(config.name.clone());

        // Build NeuroGraph with config-specific features
        let mut builder = neurograph_core::NeuroGraph::builder();

        if config.tiered_memory {
            builder = builder.tiered_memory();
        }
        if config.multigraph {
            builder = builder.multigraph();
        }
        if config.rl_forgetting {
            builder = builder.evolution();
        }

        let ng = Arc::new(builder.build().await?);

        let eval_config = LongMemEvalConfig {
            use_llm_judge: false,
            progress_interval: 100,
            max_instances,
            reset_between_instances: true,
        };

        let runner = LongMemEvalRunner::new(ng, eval_config);

        match runner.evaluate_from_file(dataset.to_str().unwrap()).await {
            Ok(report) => {
                let result = AblationResult::from_report(config.clone(), &report);
                suite.results.push(result);
            }
            Err(e) => {
                tracing::error!(config = config.id.as_str(), error = %e, "Ablation config failed");
            }
        }

        bar.inc(1);
    }

    bar.finish_with_message("Done!");

    // Print summary
    suite.print_summary();

    // Export results
    std::fs::create_dir_all(&output)?;

    suite.export_json(&output.join("ablation.json"))?;
    println!("📊 JSON   → {}", output.join("ablation.json").display());

    suite.export_jsonl(&output.join("ablation.jsonl"))?;
    println!("📊 JSONL  → {}", output.join("ablation.jsonl").display());

    suite.export_csv(&output.join("ablation.csv"))?;
    println!("📊 CSV    → {}", output.join("ablation.csv").display());

    let latex = suite.to_latex();
    std::fs::write(output.join("ablation_table.tex"), &latex)?;
    println!("📊 LaTeX  → {}", output.join("ablation_table.tex").display());

    println!("\n✅ Ablation study complete.\n");

    Ok(())
}

/// Generate tables from existing results.
fn run_tables(input: PathBuf, format: String, output: Option<PathBuf>) -> anyhow::Result<()> {
    use neurograph_eval::ablation::AblationSuite;

    let content = std::fs::read_to_string(&input)?;
    let suite: AblationSuite = serde_json::from_str(&content)?;

    match format.as_str() {
        "latex" => {
            let latex = suite.to_latex();
            if let Some(path) = output {
                std::fs::write(&path, &latex)?;
                println!("📊 LaTeX table saved to {}", path.display());
            } else {
                println!("{}", latex);
            }
        }
        "csv" => {
            let path = output.unwrap_or_else(|| PathBuf::from("ablation.csv"));
            suite.export_csv(&path)?;
            println!("📊 CSV saved to {}", path.display());
        }
        "json" => {
            let path = output.unwrap_or_else(|| PathBuf::from("ablation_pretty.json"));
            suite.export_json(&path)?;
            println!("📊 JSON saved to {}", path.display());
        }
        "jsonl" => {
            let path = output.unwrap_or_else(|| PathBuf::from("ablation.jsonl"));
            suite.export_jsonl(&path)?;
            println!("📊 JSONL saved to {}", path.display());
        }
        _ => {
            anyhow::bail!("Unknown format: {}. Use: latex, csv, json, jsonl", format);
        }
    }

    Ok(())
}

/// List all known embedding models.
fn list_models() {
    let models = neurograph_core::EmbeddingRegistry::list_all();

    println!("\n📋 NeuroGraph Embedding Model Registry");
    println!("{}", "─".repeat(72));
    println!(
        "  {:<12} {:<32} {:>8} {:>12}",
        "Provider", "Model", "Dims", "Cost/1M"
    );
    println!("{}", "─".repeat(72));

    for (provider, model, dims, cost) in &models {
        let cost_str = if *cost == 0.0 {
            "FREE".to_string()
        } else {
            format!("${:.3}", cost)
        };
        println!(
            "  {:<12} {:<32} {:>8} {:>12}",
            provider, model, dims, cost_str,
        );
    }

    println!("{}", "─".repeat(72));
    println!("  Total: {} models\n", models.len());
}

/// Export a single eval report as CSV.
fn export_report_csv(
    report: &neurograph_eval::longmemeval::EvalReport,
    path: &std::path::Path,
) -> anyhow::Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record(["instance_id", "question_type", "question", "predicted", "correct", "latency_ms"])?;

    for r in &report.results {
        wtr.write_record(&[
            &r.instance_id.to_string(),
            &r.question_type.to_string(),
            &r.question,
            &r.predicted_answer,
            &r.correct.to_string(),
            &r.latency_ms.to_string(),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}
