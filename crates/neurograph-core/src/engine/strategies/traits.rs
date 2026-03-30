// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Query strategy trait.

use async_trait::async_trait;
use std::sync::Arc;

use crate::drivers::traits::GraphDriver;
use crate::embedders::traits::Embedder;
use crate::llm::traits::LlmClient;
use crate::QueryResult;

/// Query context passed to strategies.
pub struct QueryContext {
    /// The natural language query.
    pub query: String,
    /// Graph storage driver.
    pub driver: Arc<dyn GraphDriver>,
    /// Embedding provider.
    pub embedder: Arc<dyn Embedder>,
    /// Optional LLM client for answer generation.
    pub llm: Option<Arc<dyn LlmClient>>,
    /// Group ID for multi-tenant filtering.
    pub group_id: Option<String>,
    /// Maximum cost budget for this query (USD).
    pub budget_usd: Option<f64>,
}

/// The core query strategy trait.
///
/// Each strategy implements a different approach to answering questions:
/// - Local: direct entity/subgraph retrieval
/// - Global: community summary map-reduce
/// - Temporal: time-aware graph traversal
#[async_trait]
pub trait QueryStrategy: Send + Sync {
    /// Strategy name for logging.
    fn name(&self) -> &str;

    /// Estimated cost of executing this strategy (USD).
    fn estimated_cost(&self) -> f64;

    /// Execute the query strategy and return results.
    async fn execute(&self, ctx: QueryContext) -> Result<QueryResult, QueryStrategyError>;
}

/// Errors from query strategies.
#[derive(Debug, thiserror::Error)]
pub enum QueryStrategyError {
    #[error("Retrieval error: {0}")]
    Retrieval(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("No results found")]
    NoResults,

    #[error("Strategy error: {0}")]
    Other(String),
}
