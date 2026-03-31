// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! NeuroGraph CLI — Research Paper Intelligence Platform.
//!
//! # Usage
//!
//! ```bash
//! neurograph ingest --pdf paper.pdf
//! neurograph ingest --dir ./papers/
//! neurograph search "attention mechanisms" --arxiv
//! neurograph chat
//! neurograph dashboard
//! ```

use clap::{Parser, Subcommand};
use colored::Colorize;
use neurograph_core::NeuroGraph;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "neurograph",
    version,
    about = "NeuroGraph — Research Paper Intelligence Platform",
    long_about = "A Rust-powered research paper intelligence platform with PDF ingestion,\n\
                  multi-source academic search, RAG chat, and knowledge graph visualization.\n\n\
                  Feed it PDFs → Get a searchable knowledge graph with AI chat."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Storage driver to use
    #[arg(long, default_value = "memory", global = true)]
    driver: DriverChoice,

    /// Storage path for embedded driver
    #[arg(long, global = true)]
    storage_path: Option<String>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Clone, clap::ValueEnum)]
enum DriverChoice {
    Memory,
    Embedded,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest PDFs or text into the knowledge graph
    Ingest {
        /// Text to ingest (for raw text mode)
        input: Option<String>,

        /// Ingest a single PDF file
        #[arg(long)]
        pdf: Option<PathBuf>,

        /// Ingest all PDFs in a directory
        #[arg(long)]
        dir: Option<PathBuf>,

        /// Ingest from a URL (arXiv PDFs, etc.)
        #[arg(long)]
        url: Option<String>,

        /// Embedding model (e.g., "ollama:nomic-embed-text", "openai:text-embedding-3-small")
        #[arg(long, default_value = "hash")]
        embed: String,

        /// Parse strategy: auto, fast, structured
        #[arg(long, default_value = "auto")]
        strategy: String,

        /// Treat input as a file path instead of raw text
        #[arg(short, long)]
        file: bool,
    },

    /// Query the knowledge graph
    Query {
        /// Natural language question
        question: String,

        /// Time-travel: query graph state at a specific date (ISO 8601)
        #[arg(long)]
        at: Option<String>,
    },

    /// Search for research papers across arXiv, Semantic Scholar, PubMed
    Search {
        /// Search query
        query: String,

        /// Search arXiv
        #[arg(long)]
        arxiv: bool,

        /// Search Semantic Scholar
        #[arg(long)]
        s2: bool,

        /// Search PubMed
        #[arg(long)]
        pubmed: bool,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Only papers since this year
        #[arg(long)]
        since: Option<u16>,

        /// Sort by: relevance, citations, date
        #[arg(long, default_value = "relevance")]
        sort: String,

        /// Download PDFs from results
        #[arg(long)]
        download: bool,
    },

    /// Interactive RAG chat with your knowledge graph
    Chat {
        /// LLM model to use (e.g., "ollama:llama3.2", "openai:gpt-4o")
        #[arg(long, default_value = "ollama:llama3.2")]
        model: String,
    },

    /// Start the REST API server with dashboard
    Dashboard {
        /// Port to listen on
        #[arg(short, long, default_value = "8000")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Open browser automatically
        #[arg(long)]
        open: bool,
    },

    /// Serve the REST API only (no dashboard)
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "8000")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },

    /// Show graph statistics
    Stats,

    /// Get temporal history of an entity
    History {
        /// Entity name to look up
        name: String,
    },

    /// Show what changed between two dates
    WhatChanged {
        /// Start date (ISO 8601)
        from: String,
        /// End date (ISO 8601)
        to: String,
    },

    /// Detect communities in the knowledge graph
    Communities,

    /// Run a quick inline benchmark
    Bench,

    /// Show version, configuration, and available providers
    Info,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose { "debug" } else { "warn" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter)),
        )
        .with_target(false)
        .init();

    // Build NeuroGraph instance
    let mut builder = NeuroGraph::builder();
    builder = match cli.driver {
        DriverChoice::Memory => builder.memory(),
        DriverChoice::Embedded => {
            if let Some(path) = &cli.storage_path {
                builder.embedded(path)
            } else {
                builder.embedded("neurograph_data")
            }
        }
    };
    let ng = builder.build().await?;

    match cli.command {
        Commands::Ingest { input, pdf, dir, url, embed, strategy, file } => {
            cmd_ingest(&ng, input, pdf, dir, url, &embed, &strategy, file).await?;
        }

        Commands::Query { question, at } => {
            let result = if let Some(timestamp) = at {
                let view = ng.at(&timestamp).await?;
                view.query(&question).await?
            } else {
                ng.query(&question).await?
            };

            println!("{} {}", "📝 Answer:".green().bold(), result.answer);
            println!("   Confidence: {:.0}%", result.confidence * 100.0);
            println!("   Cost: ${:.6}", result.cost_usd);
            println!("   Latency: {}ms", result.latency_ms);

            if !result.entities.is_empty() {
                println!("\n{}", "📎 Relevant entities:".cyan());
                for entity in &result.entities {
                    println!("   • {} [{}]", entity.name, entity.entity_type);
                }
            }
        }

        Commands::Search { query, arxiv, s2, pubmed, limit, since, sort, download } => {
            cmd_search(&query, arxiv, s2, pubmed, limit, since, &sort, download).await?;
        }

        Commands::Chat { model } => {
            cmd_chat(&ng, &model).await?;
        }

        Commands::Dashboard { port, host, open } => {
            cmd_dashboard(port, &host, open).await?;
        }

        Commands::Serve { port, host } => {
            cmd_serve(port, &host).await?;
        }

        Commands::Stats => {
            let stats = ng.stats().await?;
            println!("{}", "📊 NeuroGraph Statistics".cyan().bold());
            println!("───────────────────────");
            let mut sorted: Vec<_> = stats.iter().collect();
            sorted.sort_by(|(a, _), (b, _)| a.cmp(b));
            for (key, value) in sorted {
                println!("  {:<20} {}", key, value);
            }
            println!("  {:<20} ${:.6}", "total_cost_usd", ng.total_cost_usd());
        }

        Commands::History { name } => {
            let history = ng.entity_history(&name).await?;
            if history.is_empty() {
                println!("No history found for '{}'.", name);
            } else {
                println!("📅 History for '{}' ({} facts):", name, history.len());
                for rel in &history {
                    let valid = match (&rel.valid_from, &rel.valid_until) {
                        (Some(from), Some(until)) => {
                            format!("{} -> {}", from.format("%Y-%m-%d"), until.format("%Y-%m-%d"))
                        }
                        (Some(from), None) => format!("{} -> present", from.format("%Y-%m-%d")),
                        (None, _) => "unknown timeframe".to_string(),
                    };
                    let status = if rel.is_valid() { "+" } else { "-" };
                    println!("  {} {} [{}]", status, rel.fact, valid);
                }
            }
        }

        Commands::WhatChanged { from, to } => {
            let diff = ng.what_changed(&from, &to).await?;
            println!("📊 Changes from {} to {}:", from, to);
            println!("  Added entities:         {}", diff.added_entities.len());
            println!("  Modified entities:      {}", diff.modified_entities.len());
            println!("  Invalidated rels:       {}", diff.invalidated_relationships.len());
        }

        Commands::Communities => {
            let result = ng.detect_communities().await?;
            println!(
                "🌐 Community detection: {} communities found (modularity: {:.4})",
                result.communities.len(), result.modularity,
            );
            for community in &result.communities {
                println!("  • {} ({} members, level {})", community.id, community.member_entity_ids.len(), community.level);
            }
        }

        Commands::Bench => {
            cmd_bench().await?;
        }

        Commands::Info => {
            cmd_info().await;
        }
    }

    Ok(())
}

async fn cmd_ingest(
    ng: &NeuroGraph, input: Option<String>, pdf: Option<PathBuf>,
    dir: Option<PathBuf>, url: Option<String>, _embed: &str,
    strategy: &str, file: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(pdf_path) = pdf {
        println!("{}", "📄 Parsing PDF...".cyan());
        let start = std::time::Instant::now();
        let parse_strategy: neurograph_core::pdf::ParseStrategy = strategy.parse()?;
        let parser = neurograph_core::pdf::PdfParser::new(parse_strategy);
        let paper = parser.parse_paper(&pdf_path)?;

        println!("  Title:    {}", paper.title.green().bold());
        println!("  Authors:  {}", paper.authors.join(", "));
        println!("  Pages:    {}", paper.metadata.page_count);
        println!("  Chunks:   {}", paper.chunks.len());
        println!("  Sections: {}", paper.sections.len());
        println!("  Refs:     {}", paper.references.len());
        println!("  Parse:    {}ms", paper.metadata.parse_time_ms);

        let bar = indicatif::ProgressBar::new(paper.chunks.len() as u64);
        bar.set_style(indicatif::ProgressStyle::default_bar()
            .template("  Ingesting [{bar:30}] {pos}/{len} chunks")?);

        for chunk in &paper.chunks {
            ng.add_text(&chunk.text).await?;
            bar.inc(1);
        }
        bar.finish();

        let total_ms = start.elapsed().as_millis();
        println!("\n{} Ingested '{}' in {}ms", "✅".green(), paper.title, total_ms);
        return Ok(());
    }

    if let Some(dir_path) = dir {
        println!("{}", format!("📁 Scanning directory: {}", dir_path.display()).cyan());
        let parse_strategy: neurograph_core::pdf::ParseStrategy = strategy.parse()?;
        let results = neurograph_core::pdf::parse_directory(&dir_path, parse_strategy)?;

        let mut success = 0;
        let mut fail = 0;
        for result in results {
            match result {
                Ok(paper) => {
                    for chunk in &paper.chunks {
                        ng.add_text(&chunk.text).await?;
                    }
                    println!("  ✅ {} ({}p, {}ch)", paper.title, paper.metadata.page_count, paper.chunks.len());
                    success += 1;
                }
                Err(e) => {
                    println!("  ❌ Error: {}", e);
                    fail += 1;
                }
            }
        }
        println!("\n📊 {success} papers ingested, {fail} failed");
        return Ok(());
    }

    if let Some(paper_url) = url {
        println!("{}", format!("🌐 Downloading from: {}", paper_url).cyan());
        let client = reqwest::Client::new();
        let response = client.get(&paper_url).send().await?;
        let bytes = response.bytes().await?;
        let parser = neurograph_core::pdf::PdfParser::new(neurograph_core::pdf::ParseStrategy::Auto);
        let paper = parser.parse_paper_from_bytes(&bytes, &paper_url)?;
        for chunk in &paper.chunks {
            ng.add_text(&chunk.text).await?;
        }
        println!("{} Ingested '{}' ({} chunks)", "✅".green(), paper.title, paper.chunks.len());
        return Ok(());
    }

    if let Some(text_input) = input {
        let text = if file {
            tokio::fs::read_to_string(&text_input).await?
        } else {
            text_input
        };
        let episode = ng.add_text(&text).await?;
        println!("{} Ingested episode: {}", "✅".green(), episode.name);
        println!("   ID: {}", episode.id);
        println!("   Edges extracted: {}", episode.entity_edge_ids.len());
    } else {
        println!("{}", "Error: Provide --pdf, --dir, --url, or text input".red());
    }

    Ok(())
}

async fn cmd_search(
    query: &str, arxiv: bool, s2: bool, pubmed: bool,
    limit: usize, since: Option<u16>, sort: &str, _download: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut sources = Vec::new();
    if arxiv { sources.push(neurograph_core::papers::PaperSource::ArXiv); }
    if s2 { sources.push(neurograph_core::papers::PaperSource::SemanticScholar); }
    if pubmed { sources.push(neurograph_core::papers::PaperSource::PubMed); }
    if sources.is_empty() {
        sources.push(neurograph_core::papers::PaperSource::ArXiv);
        sources.push(neurograph_core::papers::PaperSource::SemanticScholar);
    }

    let sort_order = match sort {
        "citations" => neurograph_core::papers::SortOrder::Citations,
        "date" => neurograph_core::papers::SortOrder::Date,
        _ => neurograph_core::papers::SortOrder::Relevance,
    };

    let config = neurograph_core::papers::SearchConfig {
        sources, limit, since_year: since, sort_by: sort_order,
    };

    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_message(format!("Searching for '{}'...", query));
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let search = neurograph_core::papers::aggregator::UnifiedPaperSearch::new();
    let results = search.search(query, &config).await?;
    spinner.finish_and_clear();

    if results.is_empty() {
        println!("No papers found for '{}'.", query);
        return Ok(());
    }

    println!("{}", format!("🔍 {} papers found for '{}':", results.len(), query).green().bold());
    println!();

    for (i, paper) in results.iter().enumerate() {
        let citations = paper.citation_count.map(|c| format!("📊 {}cit", c)).unwrap_or_default();
        let year = paper.year.map(|y| format!("({})", y)).unwrap_or_default();

        println!("  {} {} {} {}", format!("[{}]", i + 1).cyan(), paper.title.bold(), year.dimmed(), citations.yellow());
        if !paper.authors.is_empty() {
            let authors: String = paper.authors.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
            println!("      {} — {}", authors.dimmed(), paper.source.to_string().blue());
        }
        if let Some(ref url) = paper.web_url {
            println!("      {}", url.dimmed());
        }
        println!();
    }

    Ok(())
}

async fn cmd_chat(
    _ng: &NeuroGraph, model: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (provider, model_name) = model.split_once(':').unwrap_or(("ollama", model));
    let _config = neurograph_core::chat::ChatConfig {
        model: model_name.to_string(),
        provider: provider.to_string(),
        ..Default::default()
    };

    println!("{}", "🧠 NeuroGraph Chat".green().bold());
    println!("   Model: {}:{}", provider, model_name);
    println!("   Type your question, or /quit to exit.\n");

    let mut history = neurograph_core::chat::history::ConversationHistory::new();
    loop {
        print!("  {} ", "You:".green().bold());
        use std::io::Write;
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim();
        if input.is_empty() { continue; }
        if input == "/quit" || input == "/exit" { break; }
        if input == "/clear" { history.clear(); println!("  Cleared."); continue; }

        history.add_user_message(input);
        println!("\n  {} Thinking...", "NG:".cyan().bold());
        println!("  (RAG pipeline would query the knowledge graph here)");
        println!("  Use 'neurograph ingest --pdf <file>' to add papers first.\n");
        history.add_assistant_message("(RAG response placeholder — ingest papers first)", vec![]);
    }
    Ok(())
}

async fn cmd_dashboard(port: u16, host: &str, open_browser: bool) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("http://{}:{}", host, port);
    println!("{}", "🚀 Starting NeuroGraph Dashboard".green().bold());
    println!("   API:       {}/api/v1/health", url);
    println!("   Dashboard: {}", url);

    if open_browser {
        let _ = open::that(&url);
    }

    let config = neurograph_core::server::ServerConfig {
        host: host.to_string(), port,
        cors_origins: vec!["*".to_string()],
        data_dir: None,
    };
    neurograph_core::server::start_server(config).await?;
    Ok(())
}

async fn cmd_serve(port: u16, host: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", format!("🚀 NeuroGraph API server on http://{}:{}", host, port).green().bold());
    let config = neurograph_core::server::ServerConfig {
        host: host.to_string(), port,
        cors_origins: vec!["*".to_string()],
        data_dir: None,
    };
    neurograph_core::server::start_server(config).await?;
    Ok(())
}

async fn cmd_bench() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🏃 Running NeuroGraph benchmarks...".cyan().bold());

    let ng = NeuroGraph::builder().memory().build().await?;

    let ingest_start = std::time::Instant::now();
    let n = 100;
    for i in 0..n {
        let _ = ng.add_text(&format!("Entity_{} works at Company_{} since {}", i, i % 10, 2020 + (i % 6))).await;
    }
    let ingest_time = ingest_start.elapsed();
    println!("  Ingest {} texts:    {:>8.1}ms  ({:.0} ops/sec)", n,
        ingest_time.as_secs_f64() * 1000.0, n as f64 / ingest_time.as_secs_f64());

    let query_n = 10;
    let query_start = std::time::Instant::now();
    for _ in 0..query_n {
        let _ = ng.query("Who works at Company_0?").await;
    }
    let query_time = query_start.elapsed();
    println!("  Query {} times:     {:>8.1}ms  ({:.1}ms/query)", query_n,
        query_time.as_secs_f64() * 1000.0, query_time.as_secs_f64() * 1000.0 / query_n as f64);

    let stats = ng.stats().await?;
    println!("  Graph size:         {} entities, {} relationships",
        stats.get("entities").unwrap_or(&0), stats.get("relationships").unwrap_or(&0));
    Ok(())
}

async fn cmd_info() {
    println!("{}", format!("NeuroGraph v{}", env!("CARGO_PKG_VERSION")).green().bold());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  License:    Apache-2.0");
    println!("  Repository: https://github.com/neurographai/neurograph");
    println!("  LLM:        {}", if std::env::var("OPENAI_API_KEY").is_ok() { "OpenAI" } else { "Ollama / Offline" });

    let providers = neurograph_core::embedders::available_providers();
    println!("\n  {}:", "Embedding Providers".cyan());
    for (name, description, available) in &providers {
        let status = if *available { "✅" } else { "❌" };
        println!("    {} {} — {}", status, name, description);
    }
}
