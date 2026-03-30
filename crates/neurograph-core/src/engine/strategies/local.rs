// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Local query strategy — direct entity/subgraph retrieval.
//!
//! This is the fastest and cheapest strategy. It:
//! 1. Searches for relevant entities via hybrid retrieval
//! 2. Fetches their relationships
//! 3. Builds context from found entities + relationships
//! 4. Generates answer via LLM (or returns raw context if no LLM)
//!
//! Best for: "Who is X?", "Where does X work?", direct fact lookups.

use std::time::Instant;

use async_trait::async_trait;

use crate::engine::context::{ContextBuilder, ANSWER_SYSTEM_PROMPT};
use crate::llm::traits::CompletionRequest;
use crate::retrieval::hybrid::HybridRetriever;
use crate::QueryResult;

use super::traits::{QueryContext, QueryStrategy, QueryStrategyError};

/// Local query strategy implementation.
pub struct LocalStrategy {
    /// Number of entities to retrieve.
    top_k: usize,
    /// Context token budget.
    context_tokens: usize,
}

impl LocalStrategy {
    /// Create with defaults.
    pub fn new() -> Self {
        Self {
            top_k: 10,
            context_tokens: 4000,
        }
    }

    /// Create with custom k.
    pub fn with_top_k(top_k: usize) -> Self {
        Self {
            top_k,
            context_tokens: 4000,
        }
    }
}

impl Default for LocalStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl QueryStrategy for LocalStrategy {
    fn name(&self) -> &str {
        "local"
    }

    fn estimated_cost(&self) -> f64 {
        0.003 // ~3000 tokens at gpt-4o-mini prices
    }

    async fn execute(&self, ctx: QueryContext) -> Result<QueryResult, QueryStrategyError> {
        let start = Instant::now();
        let mut total_cost = 0.0;

        // Step 1: Hybrid retrieval
        let retriever = HybridRetriever::new();
        let search_results = retriever
            .search(
                &ctx.query,
                self.top_k,
                None,
                ctx.group_id.as_deref(),
                ctx.embedder.as_ref(),
                ctx.driver.as_ref(),
            )
            .await
            .map_err(|e| QueryStrategyError::Retrieval(e.to_string()))?;

        let entities: Vec<_> = search_results.iter().map(|r| r.entity.clone()).collect();

        if entities.is_empty() {
            return Ok(QueryResult {
                answer: "No relevant information found in the knowledge graph.".to_string(),
                entities: Vec::new(),
                relationships: Vec::new(),
                communities: Vec::new(),
                confidence: 0.0,
                cost_usd: 0.0,
                latency_ms: start.elapsed().as_millis() as u64,
            });
        }

        // Step 2: Fetch relationships for found entities
        let mut all_relationships = Vec::new();
        for entity in &entities {
            if let Ok(rels) = ctx.driver.get_entity_relationships(&entity.id).await {
                all_relationships.extend(rels);
            }
        }

        // Step 3: Build context
        let context_builder = ContextBuilder::new(self.context_tokens);
        let assembled = context_builder.build(
            &entities,
            &all_relationships,
            &[],
            &ctx.query,
        );

        // Step 4: Generate answer if LLM is available
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
                Ok(response) => {
                    total_cost += response.usage.cost_usd;
                    response.content
                }
                Err(e) => {
                    tracing::warn!(error = %e, "LLM answer generation failed, returning raw context");
                    format!(
                        "Found {} relevant entities:\n{}",
                        entities.len(),
                        assembled.context_text
                    )
                }
            }
        } else {
            // No LLM — return structured context
            format!(
                "Found {} relevant entities and {} facts:\n\n{}",
                entities.len(),
                all_relationships.len(),
                assembled.context_text
            )
        };

        // Calculate confidence based on search scores
        let avg_score = if search_results.is_empty() {
            0.0
        } else {
            search_results.iter().map(|r| r.score).sum::<f64>() / search_results.len() as f64
        };

        Ok(QueryResult {
            answer,
            entities,
            relationships: all_relationships,
            communities: Vec::new(),
            confidence: (avg_score * 100.0).min(1.0), // Normalize
            cost_usd: total_cost,
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }
}
