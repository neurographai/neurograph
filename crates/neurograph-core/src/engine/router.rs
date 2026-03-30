// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Query router — classifies queries and selects optimal strategy.
//!
//! Influenced by GraphRAG's query routing (structured_search/) which
//! distinguishes between local, global, and DRIFT queries.

use std::sync::Arc;

use crate::drivers::traits::GraphDriver;
use crate::embedders::traits::Embedder;
use crate::llm::traits::LlmClient;
use crate::QueryResult;

use super::strategies::global::GlobalStrategy;
use super::strategies::local::LocalStrategy;
use super::strategies::traits::{QueryContext, QueryStrategy, QueryStrategyError};

/// Query type classification.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryType {
    /// Entity-focused query: "Who is Alice?", "Where does Bob work?"
    Local,
    /// Dataset-wide query: "What are the main topics?", "Summarize everything"
    Global,
    /// Time-specific query: "What was true in 2025?", "When did Alice move?"
    Temporal,
    /// Multi-hop query: "How is Alice connected to Bob?"
    MultiHop,
}

/// The query router.
///
/// Classifies incoming queries and dispatches to the optimal strategy.
pub struct QueryRouter {
    /// Strategy instances.
    local_strategy: LocalStrategy,
    global_strategy: GlobalStrategy,
}

impl QueryRouter {
    /// Create a new router with default strategies.
    pub fn new() -> Self {
        Self {
            local_strategy: LocalStrategy::new(),
            global_strategy: GlobalStrategy::new(),
        }
    }

    /// Classify a query into a type based on keyword analysis.
    ///
    /// Future: Use LLM for classification (cost-aware).
    pub fn classify(&self, query: &str) -> QueryType {
        let lower = query.to_lowercase();

        // Temporal indicators
        let temporal_words = [
            "when", "before", "after", "during", "history", "changed",
            "used to", "previously", "was", "were", "in 2", "in 1",
            "timeline", "evolution", "over time",
        ];
        if temporal_words.iter().any(|w| lower.contains(w)) {
            return QueryType::Temporal;
        }

        // Global indicators
        let global_words = [
            "summarize", "overview", "main topics", "all ", "everything",
            "what are the", "list all", "theme", "how many",
        ];
        if global_words.iter().any(|w| lower.contains(w)) {
            return QueryType::Global;
        }

        // Multi-hop indicators
        let multihop_words = [
            "connected", "related", "relationship between", "path",
            "how does", "influence", "impact", "through",
        ];
        if multihop_words.iter().any(|w| lower.contains(w)) {
            return QueryType::MultiHop;
        }

        // Default to local
        QueryType::Local
    }

    /// Route and execute a query.
    pub async fn execute(
        &self,
        query: &str,
        driver: Arc<dyn GraphDriver>,
        embedder: Arc<dyn Embedder>,
        llm: Option<Arc<dyn LlmClient>>,
        group_id: Option<String>,
        budget_usd: Option<f64>,
    ) -> Result<QueryResult, QueryStrategyError> {
        let query_type = self.classify(query);

        tracing::info!(
            query = query,
            query_type = ?query_type,
            "Query classified"
        );

        let context = QueryContext {
            query: query.to_string(),
            driver,
            embedder,
            llm,
            group_id,
            budget_usd,
        };

        match query_type {
            QueryType::Global => self.global_strategy.execute(context).await,
            // Local, MultiHop, and Temporal all use LocalStrategy for now.
            // Sprint 3 will add dedicated Temporal strategy.
            QueryType::Local | QueryType::MultiHop | QueryType::Temporal => {
                self.local_strategy.execute(context).await
            }
        }
    }
}

impl Default for QueryRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_classification() {
        let router = QueryRouter::new();

        assert_eq!(
            router.classify("Where does Alice work?"),
            QueryType::Local
        );
        assert_eq!(
            router.classify("When did Alice move to SF?"),
            QueryType::Temporal
        );
        assert_eq!(
            router.classify("Summarize all the topics"),
            QueryType::Global
        );
        assert_eq!(
            router.classify("How is Alice connected to Bob?"),
            QueryType::MultiHop
        );
    }
}
