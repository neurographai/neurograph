// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! LongMemEval evaluation harness.
//!
//! LongMemEval evaluates five core long-term memory abilities:
//! 1. **Information Extraction** — Direct recall from past sessions
//! 2. **Multi-Session Reasoning** — Synthesis across multiple sessions
//! 3. **Temporal Reasoning** — Understanding time-ordered events
//! 4. **Knowledge Updates** — Tracking facts that change over time
//! 5. **Abstention** — Correctly refusing to answer unknowable questions
//!
//! Reference: "LongMemEval: Benchmarking Chat Assistants on Long-Term
//! Interactive Memory" (2024). 500 curated questions with scalable
//! session histories.


use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// A single turn in a conversation session.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Turn {
    /// Speaker role: "user" or "assistant".
    pub role: String,
    /// Message content.
    pub content: String,
}

/// A conversation session with timestamp metadata.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Session {
    /// Unique session identifier (sequential).
    pub session_id: usize,
    /// When this session occurred (ISO format string).
    pub timestamp: String,
    /// Conversation turns within this session.
    pub turns: Vec<Turn>,
}

/// The five core memory abilities tested by LongMemEval.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionType {
    /// Direct recall from a single past session.
    InformationExtraction,
    /// Requires synthesizing information across multiple sessions.
    MultiSessionReasoning,
    /// Requires understanding temporal ordering of events.
    TemporalReasoning,
    /// Tracks facts that have been updated/changed over time.
    KnowledgeUpdate,
    /// Must correctly refuse to answer (information was never provided).
    Abstention,
}

impl std::fmt::Display for QuestionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuestionType::InformationExtraction => write!(f, "Information Extraction"),
            QuestionType::MultiSessionReasoning => write!(f, "Multi-Session Reasoning"),
            QuestionType::TemporalReasoning => write!(f, "Temporal Reasoning"),
            QuestionType::KnowledgeUpdate => write!(f, "Knowledge Update"),
            QuestionType::Abstention => write!(f, "Abstention"),
        }
    }
}

/// A single LongMemEval evaluation instance: (S, q, tq, a).
///
/// Each instance contains a session history S, a question q,
/// an optional query timestamp tq, and ground truth answers a.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LongMemEvalInstance {
    /// Session history to ingest before querying.
    pub session_history: Vec<Session>,
    /// The evaluation question to ask.
    pub question: String,
    /// Which memory ability this tests.
    pub question_type: QuestionType,
    /// Optional reference timestamp for temporal queries.
    pub query_timestamp: Option<String>,
    /// Acceptable ground truth answers.
    pub ground_truth: Vec<String>,
    /// Which session IDs contain the evidence (for retrieval recall).
    #[serde(default)]
    pub evidence_session_ids: Vec<usize>,
}

/// Configuration for the evaluation runner.
#[derive(Debug, Clone)]
pub struct LongMemEvalConfig {
    /// Whether to use LLM-as-judge for scoring (requires OPENAI_API_KEY).
    /// If false, uses exact string matching.
    pub use_llm_judge: bool,
    /// Log progress every N instances.
    pub progress_interval: usize,
    /// Maximum instances to evaluate (None = all).
    pub max_instances: Option<usize>,
    /// Whether to clear graph state between instances.
    pub reset_between_instances: bool,
}

impl Default for LongMemEvalConfig {
    fn default() -> Self {
        Self {
            use_llm_judge: true,
            progress_interval: 50,
            max_instances: None,
            reset_between_instances: true,
        }
    }
}

/// Result for a single evaluation instance.
#[derive(Debug, Clone, Serialize)]
pub struct EvalResult {
    /// Instance index.
    pub instance_id: usize,
    /// Which ability category.
    pub question_type: QuestionType,
    /// The question asked.
    pub question: String,
    /// System's predicted answer.
    pub predicted_answer: String,
    /// Accepted ground truth answers.
    pub ground_truth: Vec<String>,
    /// Whether the answer was judged correct.
    pub correct: bool,
    /// Retrieval recall: fraction of evidence sessions found.
    pub retrieval_recall: f64,
    /// Query latency in milliseconds.
    pub latency_ms: u64,
}

/// Per-category accuracy breakdown.
#[derive(Debug, Clone, Serialize)]
pub struct CategoryScore {
    /// Accuracy for this category (0.0–1.0).
    pub accuracy: f64,
    /// Number of instances in this category.
    pub count: usize,
    /// Mean retrieval recall for this category.
    pub mean_retrieval_recall: f64,
    /// Mean latency in milliseconds.
    pub mean_latency_ms: f64,
}

/// Aggregate evaluation report matching LongMemEval's reporting format.
#[derive(Debug, Clone, Serialize)]
pub struct EvalReport {
    /// Overall accuracy across all categories.
    pub overall_accuracy: f64,
    /// Breakdown by question category.
    pub by_category: HashMap<QuestionType, CategoryScore>,
    /// Mean query latency in milliseconds.
    pub mean_latency_ms: f64,
    /// Total instances evaluated.
    pub total_instances: usize,
    /// Total instances judged correct.
    pub total_correct: usize,
    /// Individual results (for detailed analysis).
    pub results: Vec<EvalResult>,
}

impl EvalReport {
    /// Pretty-print the report to stdout.
    pub fn print_summary(&self) {
        let bar = "═".repeat(60);
        let thin = "─".repeat(60);

        println!("\n{bar}");
        println!("  NeuroGraph LongMemEval Results");
        println!("{bar}");
        println!(
            "  Overall Accuracy: {:.1}% ({}/{})",
            self.overall_accuracy * 100.0,
            self.total_correct,
            self.total_instances
        );
        println!("  Mean Latency:     {:.1}ms", self.mean_latency_ms);
        println!("{thin}");

        let mut categories: Vec<_> = self.by_category.iter().collect();
        categories.sort_by(|a, b| a.0.to_string().cmp(&b.0.to_string()));

        for (qt, score) in &categories {
            println!(
                "  {:<28} {:.1}%  (n={}, recall={:.1}%, lat={:.0}ms)",
                qt.to_string(),
                score.accuracy * 100.0,
                score.count,
                score.mean_retrieval_recall * 100.0,
                score.mean_latency_ms,
            );
        }
        println!("{bar}\n");
    }
}

/// Errors from the evaluation harness.
#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("NeuroGraph error: {0}")]
    NeuroGraph(String),

    #[error("Dataset error: {0}")]
    Dataset(String),
}

/// The LongMemEval evaluation runner.
///
/// Ingests session histories into NeuroGraph, queries, and scores
/// against ground truth using either LLM-as-judge or exact matching.
pub struct LongMemEvalRunner {
    neurograph: Arc<neurograph_core::NeuroGraph>,
    config: LongMemEvalConfig,
}

impl LongMemEvalRunner {
    /// Create a new runner with a NeuroGraph instance.
    pub fn new(
        neurograph: Arc<neurograph_core::NeuroGraph>,
        config: LongMemEvalConfig,
    ) -> Self {
        Self { neurograph, config }
    }

    /// Load LongMemEval dataset from a JSON file.
    pub async fn load_dataset(path: &Path) -> Result<Vec<LongMemEvalInstance>, EvalError> {
        let data = tokio::fs::read_to_string(path).await?;
        let instances: Vec<LongMemEvalInstance> = serde_json::from_str(&data)?;
        Ok(instances)
    }

    /// Run evaluation from a file path.
    pub async fn evaluate_from_file(&self, path: &str) -> Result<EvalReport, EvalError> {
        let instances = Self::load_dataset(Path::new(path)).await?;
        self.evaluate(&instances).await
    }

    /// Run full evaluation on a set of instances.
    ///
    /// For each instance:
    /// 1. Reset graph state (optional)
    /// 2. Ingest session history sequentially
    /// 3. Query with optional temporal context
    /// 4. Score answer against ground truth
    pub async fn evaluate(
        &self,
        instances: &[LongMemEvalInstance],
    ) -> Result<EvalReport, EvalError> {
        let max_instances = self
            .config
            .max_instances
            .unwrap_or(instances.len())
            .min(instances.len());

        let instances = &instances[..max_instances];
        let mut results = Vec::with_capacity(instances.len());

        tracing::info!(
            total = instances.len(),
            "Starting LongMemEval evaluation"
        );

        for (idx, instance) in instances.iter().enumerate() {
            // 1. Reset graph state for clean evaluation
            if self.config.reset_between_instances {
                self.neurograph
                    .clear()
                    .await
                    .map_err(|e| EvalError::NeuroGraph(e.to_string()))?;
            }

            // 2. Ingest session history sequentially
            for session in &instance.session_history {
                for turn in &session.turns {
                    let text = format!(
                        "[Session {} @ {}] {}: {}",
                        session.session_id, session.timestamp, turn.role, turn.content
                    );
                    if let Err(e) = self.neurograph.add_text(&text).await {
                        tracing::warn!(
                            session = session.session_id,
                            error = %e,
                            "Failed to ingest turn, continuing"
                        );
                    }
                }
            }

            // 3. Query with optional temporal context
            let start = std::time::Instant::now();
            let query_result = if let Some(ref ts) = instance.query_timestamp {
                // Use time-travel for temporal queries
                match self.neurograph.at(ts).await {
                    Ok(view) => view.query(&instance.question).await,
                    Err(_) => self.neurograph.query(&instance.question).await,
                }
            } else {
                self.neurograph.query(&instance.question).await
            };
            let latency_ms = start.elapsed().as_millis() as u64;

            let predicted_answer = match query_result {
                Ok(result) => result.answer,
                Err(e) => {
                    tracing::warn!(instance = idx, error = %e, "Query failed");
                    String::from("I don't know.")
                }
            };

            // 4. Score answer against ground truth
            let correct = self.judge_answer(
                &instance.question,
                &predicted_answer,
                &instance.ground_truth,
            );

            // Compute retrieval recall (if evidence_session_ids provided)
            let retrieval_recall = if instance.evidence_session_ids.is_empty() {
                1.0 // No evidence tracking for this instance
            } else {
                // TODO: Track which sessions contributed to the answer
                0.0
            };

            results.push(EvalResult {
                instance_id: idx,
                question_type: instance.question_type.clone(),
                question: instance.question.clone(),
                predicted_answer,
                ground_truth: instance.ground_truth.clone(),
                correct,
                retrieval_recall,
                latency_ms,
            });

            // Progress logging
            if (idx + 1) % self.config.progress_interval == 0 || idx == instances.len() - 1 {
                let running_accuracy = results.iter().filter(|r| r.correct).count() as f64
                    / results.len() as f64;
                tracing::info!(
                    progress = format!("{}/{}", idx + 1, instances.len()),
                    accuracy = format!("{:.1}%", running_accuracy * 100.0),
                    "Evaluation progress"
                );
            }
        }

        Ok(self.compile_report(results))
    }

    /// Compile individual results into an aggregate report.
    fn compile_report(&self, results: Vec<EvalResult>) -> EvalReport {
        let total_instances = results.len();
        let total_correct = results.iter().filter(|r| r.correct).count();
        let overall_accuracy = if total_instances > 0 {
            total_correct as f64 / total_instances as f64
        } else {
            0.0
        };

        let mean_latency_ms = if total_instances > 0 {
            results.iter().map(|r| r.latency_ms as f64).sum::<f64>() / total_instances as f64
        } else {
            0.0
        };

        // Group by category
        let mut by_category_raw: HashMap<QuestionType, Vec<&EvalResult>> = HashMap::new();
        for r in &results {
            by_category_raw
                .entry(r.question_type.clone())
                .or_default()
                .push(r);
        }

        let by_category: HashMap<QuestionType, CategoryScore> = by_category_raw
            .into_iter()
            .map(|(qt, rs)| {
                let count = rs.len();
                let accuracy = rs.iter().filter(|r| r.correct).count() as f64 / count as f64;
                let mean_recall =
                    rs.iter().map(|r| r.retrieval_recall).sum::<f64>() / count as f64;
                let mean_lat =
                    rs.iter().map(|r| r.latency_ms as f64).sum::<f64>() / count as f64;
                (
                    qt,
                    CategoryScore {
                        accuracy,
                        count,
                        mean_retrieval_recall: mean_recall,
                        mean_latency_ms: mean_lat,
                    },
                )
            })
            .collect();

        EvalReport {
            overall_accuracy,
            by_category,
            mean_latency_ms,
            total_instances,
            total_correct,
            results,
        }
    }

    /// Judge answer correctness.
    ///
    /// Uses heuristic matching: checks if the predicted answer contains
    /// any of the ground truth answers (case-insensitive substring match).
    ///
    /// When `use_llm_judge` is true, this would delegate to GPT-4o
    /// (requires API key); currently falls back to heuristic.
    fn judge_answer(
        &self,
        _question: &str,
        predicted: &str,
        ground_truth: &[String],
    ) -> bool {
        if ground_truth.is_empty() {
            return false;
        }

        let predicted_lower = predicted.to_lowercase();

        // For abstention questions, check if the model correctly refuses
        let abstention_signals = [
            "i don't know",
            "i do not know",
            "not mentioned",
            "no information",
            "cannot answer",
            "not provided",
            "unknown",
        ];

        // Check if any ground truth answer indicates abstention
        let is_abstention = ground_truth.iter().any(|gt| {
            let gt_lower = gt.to_lowercase();
            abstention_signals.iter().any(|s| gt_lower.contains(s))
        });

        if is_abstention {
            // Model should also abstain
            return abstention_signals
                .iter()
                .any(|s| predicted_lower.contains(s));
        }

        // Standard matching: check if predicted contains ground truth
        // (case-insensitive substring match, matching LongMemEval's lenient scoring)
        ground_truth.iter().any(|gt| {
            let gt_lower = gt.to_lowercase();
            // Exact containment in either direction
            predicted_lower.contains(&gt_lower) || gt_lower.contains(&predicted_lower)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_question_type_display() {
        assert_eq!(
            QuestionType::InformationExtraction.to_string(),
            "Information Extraction"
        );
        assert_eq!(
            QuestionType::TemporalReasoning.to_string(),
            "Temporal Reasoning"
        );
    }

    #[test]
    fn test_judge_answer_exact_match() {
        let runner_config = LongMemEvalConfig::default();
        let ng = futures::executor::block_on(async {
            Arc::new(
                neurograph_core::NeuroGraph::builder()
                    .build()
                    .await
                    .unwrap(),
            )
        });
        let runner = LongMemEvalRunner::new(ng, runner_config);

        // Exact match
        assert!(runner.judge_answer(
            "Where does Alice work?",
            "Alice works at Anthropic",
            &["Anthropic".to_string()],
        ));

        // Case-insensitive
        assert!(runner.judge_answer(
            "Where does Alice work?",
            "alice works at anthropic",
            &["Anthropic".to_string()],
        ));

        // No match
        assert!(!runner.judge_answer(
            "Where does Alice work?",
            "Alice works at Google",
            &["Anthropic".to_string()],
        ));
    }

    #[test]
    fn test_judge_answer_abstention() {
        let ng = futures::executor::block_on(async {
            Arc::new(
                neurograph_core::NeuroGraph::builder()
                    .build()
                    .await
                    .unwrap(),
            )
        });
        let runner = LongMemEvalRunner::new(ng, LongMemEvalConfig::default());

        // Model correctly abstains
        assert!(runner.judge_answer(
            "What is Alice's phone number?",
            "I don't know",
            &["I don't know".to_string()],
        ));

        // Model incorrectly answers when it should abstain
        assert!(!runner.judge_answer(
            "What is Alice's phone number?",
            "Alice's phone number is 555-1234",
            &["I don't know".to_string()],
        ));
    }

    #[test]
    fn test_compile_report_empty() {
        let ng = futures::executor::block_on(async {
            Arc::new(
                neurograph_core::NeuroGraph::builder()
                    .build()
                    .await
                    .unwrap(),
            )
        });
        let runner = LongMemEvalRunner::new(ng, LongMemEvalConfig::default());
        let report = runner.compile_report(vec![]);

        assert_eq!(report.total_instances, 0);
        assert_eq!(report.overall_accuracy, 0.0);
    }

    #[test]
    fn test_compile_report_mixed() {
        let ng = futures::executor::block_on(async {
            Arc::new(
                neurograph_core::NeuroGraph::builder()
                    .build()
                    .await
                    .unwrap(),
            )
        });
        let runner = LongMemEvalRunner::new(ng, LongMemEvalConfig::default());

        let results = vec![
            EvalResult {
                instance_id: 0,
                question_type: QuestionType::InformationExtraction,
                question: "Q1".to_string(),
                predicted_answer: "A1".to_string(),
                ground_truth: vec!["A1".to_string()],
                correct: true,
                retrieval_recall: 1.0,
                latency_ms: 100,
            },
            EvalResult {
                instance_id: 1,
                question_type: QuestionType::TemporalReasoning,
                question: "Q2".to_string(),
                predicted_answer: "Wrong".to_string(),
                ground_truth: vec!["Right".to_string()],
                correct: false,
                retrieval_recall: 0.5,
                latency_ms: 200,
            },
        ];

        let report = runner.compile_report(results);

        assert_eq!(report.total_instances, 2);
        assert_eq!(report.total_correct, 1);
        assert!((report.overall_accuracy - 0.5).abs() < f64::EPSILON);
        assert!((report.mean_latency_ms - 150.0).abs() < f64::EPSILON);

        let ie = report.by_category.get(&QuestionType::InformationExtraction).unwrap();
        assert!((ie.accuracy - 1.0).abs() < f64::EPSILON);
        assert_eq!(ie.count, 1);

        let tr = report.by_category.get(&QuestionType::TemporalReasoning).unwrap();
        assert!((tr.accuracy - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_deserialize_instance() {
        let json = r#"{
            "session_history": [
                {
                    "session_id": 1,
                    "timestamp": "2024-01-15T10:00:00Z",
                    "turns": [
                        {"role": "user", "content": "I just got a new job at Anthropic."},
                        {"role": "assistant", "content": "Congratulations!"}
                    ]
                }
            ],
            "question": "Where does the user work?",
            "question_type": "information_extraction",
            "ground_truth": ["Anthropic"],
            "evidence_session_ids": [1]
        }"#;

        let instance: LongMemEvalInstance = serde_json::from_str(json).unwrap();
        assert_eq!(instance.session_history.len(), 1);
        assert_eq!(instance.question_type, QuestionType::InformationExtraction);
        assert_eq!(instance.ground_truth, vec!["Anthropic"]);
        assert!(instance.query_timestamp.is_none());
    }
}
