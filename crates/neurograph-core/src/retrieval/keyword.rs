// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Keyword (BM25-style) search over the knowledge graph.

use crate::drivers::traits::GraphDriver;

use super::semantic::ScoredEntity;

/// Keyword search: finds entities by text matching.
pub struct KeywordSearcher;

impl KeywordSearcher {
    /// Search for entities matching the query text.
    ///
    /// Uses the driver's text search (which may use exact matching,
    /// TF-IDF, or BM25 depending on the driver implementation).
    pub async fn search(
        query: &str,
        k: usize,
        group_id: Option<&str>,
        driver: &dyn GraphDriver,
    ) -> Result<Vec<ScoredEntity>, KeywordSearchError> {
        let results = driver
            .search_entities_by_text(query, k, group_id)
            .await
            .map_err(|e| KeywordSearchError::DriverError(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|r| ScoredEntity {
                entity: r.entity,
                score: r.score,
                source: "keyword".to_string(),
            })
            .collect())
    }
}

/// Errors from keyword search.
#[derive(Debug, thiserror::Error)]
pub enum KeywordSearchError {
    #[error("Driver error: {0}")]
    DriverError(String),
}
