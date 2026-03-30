// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Global query strategy — community-based map-reduce.
//!
//! This strategy is designed for dataset-wide queries like:
//! "What are the main topics?", "Summarize everything", "How many people?"
//!
//! It works by:
//! 1. Retrieving all community summaries at the top hierarchy level
//! 2. Map: Send each community summary + query to the LLM for a partial answer
//! 3. Reduce: Combine all partial answers into a final answer
//!
//! Influenced by GraphRAG's global search (structured_search/global_search.py)
//! which uses community map-reduce for holistic dataset queries.

use std::time::Instant;

use async_trait::async_trait;

use crate::engine::context::{ContextBuilder, ANSWER_SYSTEM_PROMPT};
use crate::llm::traits::CompletionRequest;
use crate::retrieval::hybrid::HybridRetriever;
use crate::QueryResult;

use super::traits::{QueryContext, QueryStrategy, QueryStrategyError};

/// Global query strategy — community map-reduce.
///
/// Best for: "What are the main topics?", "Summarize everything",
///           "Give me an overview", "How many entities are there?"
#[allow(dead_code)]
pub struct GlobalStrategy {
    /// Maximum number of communities to process.
    max_communities: usize,
    /// Token budget for each community's context.
    context_tokens_per_community: usize,
}

impl GlobalStrategy {
    /// Create with defaults.
    pub fn new() -> Self {
        Self {
            max_communities: 20,
            context_tokens_per_community: 2000,
        }
    }

    /// Create with custom limits.
    pub fn with_limits(max_communities: usize, context_tokens: usize) -> Self {
        Self {
            max_communities,
            context_tokens_per_community: context_tokens,
        }
    }
}

impl Default for GlobalStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl QueryStrategy for GlobalStrategy {
    fn name(&self) -> &str {
        "global"
    }

    fn estimated_cost(&self) -> f64 {
        // Roughly: max_communities * cost_per_community + reduce_cost
        0.02 // ~$0.02 for a modest graph
    }

    async fn execute(&self, ctx: QueryContext) -> Result<QueryResult, QueryStrategyError> {
        let start = Instant::now();
        let mut total_cost = 0.0;

        // Step 1: Get community summaries from the top level
        let communities = ctx
            .driver
            .get_communities_at_level(0)
            .await
            .map_err(|e| QueryStrategyError::Retrieval(e.to_string()))?;

        // If no communities exist yet, fall back to hybrid search
        if communities.is_empty() {
            tracing::info!("No communities found, falling back to hybrid search for global query");
            return self.fallback_hybrid(&ctx, start).await;
        }

        // Cap the number of communities to process
        let communities: Vec<_> = communities
            .into_iter()
            .take(self.max_communities)
            .collect();

        // Step 2: Map phase — generate partial answers per community
        let mut partial_answers = Vec::new();

        if let Some(ref llm) = ctx.llm {
            for community in &communities {
                if community.summary.is_empty() {
                    continue;
                }

                let prompt = format!(
                    "Community: {}\nSummary: {}\n\nQuestion: {}\n\n\
                     Based on this community's information, provide a partial answer to the question.\
                     If the community is not relevant, say \"NOT RELEVANT\".",
                    community.name, community.summary, ctx.query
                );

                let request = CompletionRequest::new(prompt)
                    .with_system(ANSWER_SYSTEM_PROMPT)
                    .with_temperature(0.0)
                    .with_max_tokens(300);

                match llm.complete(request).await {
                    Ok(response) => {
                        total_cost += response.usage.cost_usd;
                        let content = response.content.trim().to_string();
                        if !content.to_uppercase().contains("NOT RELEVANT") {
                            partial_answers.push(content);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            community = %community.name,
                            error = %e,
                            "Map phase LLM call failed"
                        );
                    }
                }
            }

            // Step 3: Reduce phase — combine partial answers
            if !partial_answers.is_empty() {
                let reduce_prompt = format!(
                    "You have gathered the following partial answers from different \
                     knowledge communities:\n\n{}\n\n\
                     Original question: {}\n\n\
                     Synthesize these into a single comprehensive answer.",
                    partial_answers
                        .iter()
                        .enumerate()
                        .map(|(i, a)| format!("{}. {}", i + 1, a))
                        .collect::<Vec<_>>()
                        .join("\n\n"),
                    ctx.query
                );

                let reduce_request = CompletionRequest::new(reduce_prompt)
                    .with_system(ANSWER_SYSTEM_PROMPT)
                    .with_temperature(0.1)
                    .with_max_tokens(600);

                match llm.complete(reduce_request).await {
                    Ok(response) => {
                        total_cost += response.usage.cost_usd;
                        return Ok(QueryResult {
                            answer: response.content,
                            entities: Vec::new(),
                            relationships: Vec::new(),
                            communities: communities.clone(),
                            confidence: 0.8,
                            cost_usd: total_cost,
                            latency_ms: start.elapsed().as_millis() as u64,
                        });
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Reduce phase failed");
                    }
                }
            }
        }

        // No LLM or LLM failed — return community summaries as context
        let summary_text: String = communities
            .iter()
            .filter(|c| !c.summary.is_empty())
            .map(|c| format!("**{}**: {}", c.name, c.summary))
            .collect::<Vec<_>>()
            .join("\n\n");

        let answer = if summary_text.is_empty() {
            "No community summaries available for a global answer.".to_string()
        } else {
            format!(
                "Found {} communities:\n\n{}",
                communities.len(),
                summary_text
            )
        };

        Ok(QueryResult {
            answer,
            entities: Vec::new(),
            relationships: Vec::new(),
            communities,
            confidence: 0.5,
            cost_usd: total_cost,
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }
}

impl GlobalStrategy {
    /// Fallback to hybrid search when no communities exist.
    async fn fallback_hybrid(
        &self,
        ctx: &QueryContext,
        start: Instant,
    ) -> Result<QueryResult, QueryStrategyError> {
        let retriever = HybridRetriever::new();
        let search_results = retriever
            .search(
                &ctx.query,
                20,
                None,
                ctx.group_id.as_deref(),
                ctx.embedder.as_ref(),
                ctx.driver.as_ref(),
            )
            .await
            .map_err(|e| QueryStrategyError::Retrieval(e.to_string()))?;

        let entities: Vec<_> = search_results.iter().map(|r| r.entity.clone()).collect();

        let context_builder = ContextBuilder::new(4000);
        let assembled = context_builder.build(&entities, &[], &[], &ctx.query);

        let answer = if let Some(ref llm) = ctx.llm {
            let prompt = format!(
                "Context from knowledge graph:\n{}\n\nQuestion: {}",
                assembled.context_text, ctx.query
            );
            let request = CompletionRequest::new(prompt)
                .with_system(ANSWER_SYSTEM_PROMPT)
                .with_temperature(0.1)
                .with_max_tokens(500);

            match llm.complete(request).await {
                Ok(response) => response.content,
                Err(_) => assembled.context_text,
            }
        } else {
            assembled.context_text
        };

        Ok(QueryResult {
            answer,
            entities,
            relationships: Vec::new(),
            communities: Vec::new(),
            confidence: 0.5,
            cost_usd: 0.0,
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }
}
