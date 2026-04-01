// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! NeuroGraph Chat Agent — the orchestrator.
//!
//! 7-step `process()` flow:
//! 1. Classify intent
//! 2. Plan tools
//! 3. Execute tools (parallel where safe)
//! 4. Build context from results
//! 5. Generate answer via LLM
//! 6. Compose structured AgentResponse
//! 7. Update session

use std::sync::Arc;
use std::time::Instant;

use super::history::ConversationHistory;
use super::intent::{ChatIntent, ClassifiedIntent, IntentClassifier};
use super::response::{
    AgentResponse, EvidenceChunk, FollowUpQuestion, GraphAction, ResponseMeta,
};
use super::tools::{AgentTool, ToolExecutor, ToolPlanner, ToolResult};
use crate::llm::registry::TaskType;
use crate::llm::router::LlmRouter;
use crate::llm::traits::{CompletionRequest, LlmUsage};

/// The main chat agent.
pub struct NeuroGraphAgent {
    graph: Arc<crate::NeuroGraph>,
    router: Arc<LlmRouter>,
    classifier: IntentClassifier,
}

impl NeuroGraphAgent {
    /// Create a new agent.
    pub fn new(graph: Arc<crate::NeuroGraph>, router: Arc<LlmRouter>) -> Self {
        Self {
            graph,
            router,
            classifier: IntentClassifier::new(),
        }
    }

    /// Process a user message through the full agent loop.
    pub async fn process(
        &self,
        message: &str,
        session_id: &str,
        history: &mut ConversationHistory,
    ) -> anyhow::Result<AgentResponse> {
        let start = Instant::now();

        // ── Step 1: Classify intent ─────────────────────────────
        let classified = self.classifier.classify(message);
        tracing::info!(
            intent = %classified.intent,
            confidence = classified.confidence,
            method = ?classified.method,
            "Intent classified"
        );

        // ── Step 2: Plan tools ──────────────────────────────────
        let tools = ToolPlanner::plan(
            &classified.intent,
            message,
            &classified.extracted_entities,
        );
        tracing::debug!(tools = ?tools.iter().map(|t| t.name()).collect::<Vec<_>>(), "Tools planned");

        // ── Step 3: Execute tools ───────────────────────────────
        let tool_results = self.execute_tools(&tools).await;
        let tools_used: Vec<String> = tool_results.iter().map(|r| r.tool_name.clone()).collect();

        // ── Step 4: Build context ───────────────────────────────
        let (context_text, evidence, graph_actions) = self.build_context(&tool_results);

        // ── Step 5: Generate answer via LLM ─────────────────────
        let (answer, confidence, llm_usage) = self
            .generate_answer(message, &context_text, &classified, history)
            .await?;

        // ── Step 6: Generate follow-ups ─────────────────────────
        let follow_ups = self.generate_follow_ups(&classified, &answer);

        // ── Step 7: Update history ──────────────────────────────
        history.add_user_message(message);
        history.add_assistant_message(&answer, vec![]);

        let latency_ms = start.elapsed().as_millis() as u64;

        Ok(AgentResponse {
            answer,
            confidence,
            evidence,
            graph_actions,
            follow_ups,
            meta: ResponseMeta {
                intent: classified,
                tools_used,
                model_used: llm_usage.model.clone(),
                input_tokens: llm_usage.input_tokens,
                output_tokens: llm_usage.output_tokens,
                cost_usd: llm_usage.cost_usd,
                latency_ms,
                session_id: session_id.to_string(),
            },
        })
    }

    /// Execute a list of tools, parallelizing where safe.
    async fn execute_tools(&self, tools: &[AgentTool]) -> Vec<ToolResult> {
        let mut results = Vec::new();

        // Split into parallel-safe and sequential tools
        let (parallel, sequential): (Vec<_>, Vec<_>) =
            tools.iter().partition(|t| t.is_parallel_safe());

        // Run parallel tools concurrently
        if !parallel.is_empty() {
            let futures: Vec<_> = parallel
                .iter()
                .map(|tool| ToolExecutor::execute(tool, &self.graph))
                .collect();

            let parallel_results = futures::future::join_all(futures).await;
            for result in parallel_results {
                match result {
                    Ok(r) => results.push(r),
                    Err(e) => {
                        tracing::warn!("Tool execution failed: {}", e);
                    }
                }
            }
        }

        // Run sequential tools in order
        for tool in sequential {
            match ToolExecutor::execute(tool, &self.graph).await {
                Ok(r) => results.push(r),
                Err(e) => {
                    tracing::warn!("Tool execution failed: {}", e);
                }
            }
        }

        results
    }

    /// Build context from tool results.
    fn build_context(
        &self,
        results: &[ToolResult],
    ) -> (String, Vec<EvidenceChunk>, Vec<GraphAction>) {
        let mut context_parts = Vec::new();
        let mut evidence = Vec::new();
        let mut graph_actions = Vec::new();

        for result in results {
            if !result.context_text.is_empty() {
                context_parts.push(format!(
                    "[{}]\n{}",
                    result.tool_name, result.context_text
                ));
            }
            evidence.extend(result.evidence.clone());
            graph_actions.extend(result.graph_actions.clone());
        }

        // Deduplicate evidence by text prefix
        evidence.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));
        evidence.truncate(10);

        let context_text = context_parts.join("\n\n---\n\n");
        (context_text, evidence, graph_actions)
    }

    /// Generate the final answer using the LLM.
    async fn generate_answer(
        &self,
        question: &str,
        context: &str,
        classified: &ClassifiedIntent,
        history: &ConversationHistory,
    ) -> anyhow::Result<(String, f32, LlmUsage)> {
        // Pick the right task type for routing
        let task = match classified.intent {
            ChatIntent::Summarize | ChatIntent::DiscoverThemes => TaskType::CommunitySummary,
            ChatIntent::FindContradictions => TaskType::ConflictDetection,
            ChatIntent::TemporalCompare | ChatIntent::TimeTravel => TaskType::TemporalAnalysis,
            _ => TaskType::RagGeneration,
        };

        let system_prompt = self.build_system_prompt(&classified.intent);

        // Build conversation context
        let recent_history = history.last_n(4);
        let mut history_text = String::new();
        for msg in recent_history {
            history_text.push_str(&format!(
                "{}: {}\n",
                match msg.role {
                    super::Role::User => "User",
                    super::Role::Assistant => "Assistant",
                    super::Role::System => "System",
                },
                msg.content
            ));
        }

        let user_prompt = if context.is_empty() {
            format!(
                "{}Question: {}",
                if history_text.is_empty() {
                    String::new()
                } else {
                    format!("Recent conversation:\n{}\n---\n\n", history_text)
                },
                question
            )
        } else {
            format!(
                "{}Context from knowledge graph:\n\n{}\n\n---\n\nQuestion: {}",
                if history_text.is_empty() {
                    String::new()
                } else {
                    format!("Recent conversation:\n{}\n---\n\n", history_text)
                },
                context,
                question
            )
        };

        // Route to best LLM for this task
        match self.router.route(task).await {
            Ok(client) => {
                let request = CompletionRequest::new(&user_prompt)
                    .with_system(&system_prompt)
                    .with_temperature(0.3)
                    .with_max_tokens(2048);

                match client.complete(request).await {
                    Ok(response) => {
                        // Record usage
                        let tracker = &self.router.token_tracker;
                        tracker
                            .record(
                                crate::llm::PromptType::AnswerGeneration,
                                response.usage.input_tokens as u64,
                                response.usage.output_tokens as u64,
                                response.usage.cost_usd,
                            )
                            .await;

                        Ok((
                            response.content,
                            classified.confidence,
                            response.usage,
                        ))
                    }
                    Err(e) => {
                        tracing::warn!("LLM call failed: {}, using fallback answer", e);
                        Ok(self.fallback_answer(context, classified))
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Router failed: {}, using fallback answer", e);
                Ok(self.fallback_answer(context, classified))
            }
        }
    }

    /// Build a task-specific system prompt.
    fn build_system_prompt(&self, intent: &ChatIntent) -> String {
        let base = "You are NeuroGraph, an intelligent research paper knowledge assistant. \
                     You answer questions based on the provided context from a knowledge graph \
                     of research papers. Always cite evidence when available. Be precise and \
                     informative.";

        let intent_guidance = match intent {
            ChatIntent::Explain => " Focus on clear, educational explanations.",
            ChatIntent::Explore => " Describe connections and relationships between entities.",
            ChatIntent::TemporalCompare => " Compare how things have changed over time.",
            ChatIntent::TimeTravel => " Describe the state of knowledge at the specified time.",
            ChatIntent::FindContradictions => " Identify and explain any conflicting information.",
            ChatIntent::Summarize => " Provide a concise, structured summary.",
            ChatIntent::Search => " List relevant results clearly.",
            ChatIntent::TraceRelationship => " Trace and explain the chain of relationships.",
            ChatIntent::DiscoverThemes => " Identify and describe the major themes and clusters.",
            ChatIntent::FilterGraph => " Describe the filtered view and relevant entities.",
            ChatIntent::General => " Be helpful and conversational.",
        };

        format!("{}{}", base, intent_guidance)
    }

    /// Fallback answer when LLM is unavailable.
    fn fallback_answer(
        &self,
        context: &str,
        classified: &ClassifiedIntent,
    ) -> (String, f32, LlmUsage) {
        let answer = if context.is_empty() {
            "I found relevant information in the knowledge graph but couldn't generate a \
             detailed answer right now. Try configuring an LLM provider in Settings."
                .to_string()
        } else {
            format!(
                "Based on the knowledge graph, here's what I found:\n\n{}",
                &context[..context.len().min(500)]
            )
        };

        (
            answer,
            classified.confidence * 0.5,
            LlmUsage {
                input_tokens: 0,
                output_tokens: 0,
                cost_usd: 0.0,
                latency_ms: 0,
                model: "fallback".to_string(),
            },
        )
    }

    /// Generate follow-up question suggestions.
    fn generate_follow_ups(
        &self,
        classified: &ClassifiedIntent,
        _answer: &str,
    ) -> Vec<FollowUpQuestion> {
        match classified.intent {
            ChatIntent::Explain => vec![
                FollowUpQuestion {
                    question: "How does this relate to other concepts?".to_string(),
                    intent_hint: ChatIntent::Explore,
                    rationale: "Explore connections".to_string(),
                },
                FollowUpQuestion {
                    question: "Has this changed over time?".to_string(),
                    intent_hint: ChatIntent::TemporalCompare,
                    rationale: "Temporal analysis".to_string(),
                },
                FollowUpQuestion {
                    question: "Summarize the key papers on this topic".to_string(),
                    intent_hint: ChatIntent::Summarize,
                    rationale: "Broader context".to_string(),
                },
            ],
            ChatIntent::Explore => vec![
                FollowUpQuestion {
                    question: "Explain this entity in more detail".to_string(),
                    intent_hint: ChatIntent::Explain,
                    rationale: "Deeper understanding".to_string(),
                },
                FollowUpQuestion {
                    question: "What themes emerge from these connections?".to_string(),
                    intent_hint: ChatIntent::DiscoverThemes,
                    rationale: "Pattern discovery".to_string(),
                },
            ],
            ChatIntent::Summarize => vec![
                FollowUpQuestion {
                    question: "Are there any contradictions?".to_string(),
                    intent_hint: ChatIntent::FindContradictions,
                    rationale: "Validity check".to_string(),
                },
                FollowUpQuestion {
                    question: "Show me the underlying graph".to_string(),
                    intent_hint: ChatIntent::Explore,
                    rationale: "Visual exploration".to_string(),
                },
            ],
            _ => vec![FollowUpQuestion {
                question: "Tell me more about this".to_string(),
                intent_hint: ChatIntent::Explain,
                rationale: "Continue exploration".to_string(),
            }],
        }
    }

    /// Classify intent only (for the UI preview endpoint).
    pub fn classify_intent(&self, message: &str) -> ClassifiedIntent {
        self.classifier.classify(message)
    }
}
