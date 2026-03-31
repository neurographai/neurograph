// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Per-prompt-type token usage tracking.
//!
//! Tracks LLM token consumption at a granular level — by prompt type
//! (entity extraction, relationship extraction, summarization, etc.).
//! Closes a gap where Graphiti has `token_tracker` with per-type
//! usage summaries while NeuroGraph only tracked total cost.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;

/// Identifiers for different types of LLM prompts.
///
/// Allows tracking token usage independently per operation type,
/// enabling fine-grained cost analysis and optimization.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum PromptType {
    /// Entity extraction from raw text.
    EntityExtraction,
    /// Relationship extraction from raw text.
    RelationshipExtraction,
    /// Entity resolution (deduplication).
    EntityResolution,
    /// Community summary generation.
    CommunitySummary,
    /// Query rewriting/expansion.
    QueryRewrite,
    /// Final answer generation.
    AnswerGeneration,
    /// Cross-encoder reranking.
    Reranking,
    /// Conflict resolution.
    ConflictResolution,
    /// Custom prompt type.
    Custom(String),
}

impl std::fmt::Display for PromptType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PromptType::EntityExtraction => write!(f, "entity_extraction"),
            PromptType::RelationshipExtraction => write!(f, "relationship_extraction"),
            PromptType::EntityResolution => write!(f, "entity_resolution"),
            PromptType::CommunitySummary => write!(f, "community_summary"),
            PromptType::QueryRewrite => write!(f, "query_rewrite"),
            PromptType::AnswerGeneration => write!(f, "answer_generation"),
            PromptType::Reranking => write!(f, "reranking"),
            PromptType::ConflictResolution => write!(f, "conflict_resolution"),
            PromptType::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

/// Token usage for a single prompt type.
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    /// Total input tokens consumed.
    pub input_tokens: u64,
    /// Total output tokens consumed.
    pub output_tokens: u64,
    /// Number of API calls made.
    pub call_count: u64,
    /// Estimated cost in USD.
    pub estimated_cost_usd: f64,
}

impl TokenUsage {
    /// Total tokens (input + output).
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }

    /// Average tokens per call.
    pub fn avg_tokens_per_call(&self) -> f64 {
        if self.call_count == 0 {
            0.0
        } else {
            self.total_tokens() as f64 / self.call_count as f64
        }
    }
}

/// Tracks LLM token usage per prompt type.
///
/// Thread-safe via atomic counters and async RwLock.
///
/// # Example
///
/// ```rust
/// use neurograph_core::llm::token_tracker::{TokenTracker, PromptType};
///
/// let tracker = TokenTracker::new();
///
/// // Record usage
/// // tracker.record(PromptType::EntityExtraction, 500, 200, 0.0035).await;
/// // tracker.record(PromptType::AnswerGeneration, 1000, 500, 0.015).await;
///
/// // Get summary
/// // let summary = tracker.get_summary().await;
/// // tracker.print_summary().await;
/// ```
pub struct TokenTracker {
    /// Per-prompt-type usage tracking.
    usage: RwLock<HashMap<PromptType, TokenUsage>>,
    /// Global total input tokens (atomic for lock-free reads).
    total_input: AtomicU64,
    /// Global total output tokens (atomic for lock-free reads).
    total_output: AtomicU64,
    /// Global total API calls.
    total_calls: AtomicU64,
}

impl TokenTracker {
    /// Create a new empty token tracker.
    pub fn new() -> Self {
        Self {
            usage: RwLock::new(HashMap::new()),
            total_input: AtomicU64::new(0),
            total_output: AtomicU64::new(0),
            total_calls: AtomicU64::new(0),
        }
    }

    /// Record token usage for a specific prompt type.
    pub async fn record(
        &self,
        prompt_type: PromptType,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
    ) {
        self.total_input.fetch_add(input_tokens, Ordering::Relaxed);
        self.total_output.fetch_add(output_tokens, Ordering::Relaxed);
        self.total_calls.fetch_add(1, Ordering::Relaxed);

        let mut usage = self.usage.write().await;
        let entry = usage.entry(prompt_type).or_default();
        entry.input_tokens += input_tokens;
        entry.output_tokens += output_tokens;
        entry.call_count += 1;
        entry.estimated_cost_usd += cost_usd;
    }

    /// Get usage for a specific prompt type.
    pub async fn get_usage(&self, prompt_type: &PromptType) -> Option<TokenUsage> {
        self.usage.read().await.get(prompt_type).cloned()
    }

    /// Get a complete summary of all prompt types.
    pub async fn get_summary(&self) -> HashMap<PromptType, TokenUsage> {
        self.usage.read().await.clone()
    }

    /// Get global totals (input_tokens, output_tokens, call_count).
    pub fn totals(&self) -> (u64, u64, u64) {
        (
            self.total_input.load(Ordering::Relaxed),
            self.total_output.load(Ordering::Relaxed),
            self.total_calls.load(Ordering::Relaxed),
        )
    }

    /// Get total estimated cost across all prompt types.
    pub async fn total_cost(&self) -> f64 {
        self.usage
            .read()
            .await
            .values()
            .map(|u| u.estimated_cost_usd)
            .sum()
    }

    /// Print a formatted summary table to stdout.
    pub async fn print_summary(&self) {
        let summary = self.get_summary().await;
        let (total_in, total_out, total_calls) = self.totals();
        let total_cost = self.total_cost().await;

        println!("┌──────────────────────────────┬──────────┬──────────┬────────┬──────────┐");
        println!("│ Prompt Type                  │ Input    │ Output   │ Calls  │ Cost USD │");
        println!("├──────────────────────────────┼──────────┼──────────┼────────┼──────────┤");

        let mut entries: Vec<_> = summary.iter().collect();
        entries.sort_by(|a, b| b.1.total_tokens().cmp(&a.1.total_tokens()));

        for (prompt_type, usage) in &entries {
            println!(
                "│ {:<28} │ {:>8} │ {:>8} │ {:>6} │ ${:>7.4} │",
                format!("{}", prompt_type),
                usage.input_tokens,
                usage.output_tokens,
                usage.call_count,
                usage.estimated_cost_usd,
            );
        }

        println!("├──────────────────────────────┼──────────┼──────────┼────────┼──────────┤");
        println!(
            "│ {:<28} │ {:>8} │ {:>8} │ {:>6} │ ${:>7.4} │",
            "TOTAL", total_in, total_out, total_calls, total_cost,
        );
        println!("└──────────────────────────────┴──────────┴──────────┴────────┴──────────┘");
    }

    /// Reset all tracking data.
    pub async fn reset(&self) {
        self.usage.write().await.clear();
        self.total_input.store(0, Ordering::Relaxed);
        self.total_output.store(0, Ordering::Relaxed);
        self.total_calls.store(0, Ordering::Relaxed);
    }
}

impl Default for TokenTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_and_get() {
        let tracker = TokenTracker::new();

        tracker
            .record(PromptType::EntityExtraction, 500, 200, 0.0035)
            .await;
        tracker
            .record(PromptType::EntityExtraction, 300, 100, 0.0020)
            .await;

        let usage = tracker.get_usage(&PromptType::EntityExtraction).await.unwrap();
        assert_eq!(usage.input_tokens, 800);
        assert_eq!(usage.output_tokens, 300);
        assert_eq!(usage.call_count, 2);
        assert!((usage.estimated_cost_usd - 0.0055).abs() < 1e-10);
    }

    #[tokio::test]
    async fn test_totals() {
        let tracker = TokenTracker::new();

        tracker
            .record(PromptType::EntityExtraction, 500, 200, 0.01)
            .await;
        tracker
            .record(PromptType::AnswerGeneration, 1000, 500, 0.02)
            .await;

        let (input, output, calls) = tracker.totals();
        assert_eq!(input, 1500);
        assert_eq!(output, 700);
        assert_eq!(calls, 2);
    }

    #[tokio::test]
    async fn test_total_cost() {
        let tracker = TokenTracker::new();

        tracker.record(PromptType::EntityExtraction, 500, 200, 0.01).await;
        tracker.record(PromptType::AnswerGeneration, 1000, 500, 0.02).await;

        let cost = tracker.total_cost().await;
        assert!((cost - 0.03).abs() < 1e-10);
    }

    #[tokio::test]
    async fn test_reset() {
        let tracker = TokenTracker::new();
        tracker.record(PromptType::Reranking, 100, 50, 0.001).await;

        tracker.reset().await;

        let (input, output, calls) = tracker.totals();
        assert_eq!(input, 0);
        assert_eq!(output, 0);
        assert_eq!(calls, 0);
        assert!(tracker.get_summary().await.is_empty());
    }

    #[test]
    fn test_token_usage_helpers() {
        let usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 500,
            call_count: 10,
            estimated_cost_usd: 0.05,
        };

        assert_eq!(usage.total_tokens(), 1500);
        assert!((usage.avg_tokens_per_call() - 150.0).abs() < f64::EPSILON);
    }
}
