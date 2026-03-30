// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Embedder trait definition.

use async_trait::async_trait;

/// Errors from embedding operations.
#[derive(Debug, thiserror::Error)]
pub enum EmbedderError {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Model not available: {0}")]
    ModelNotAvailable(String),
}

pub type EmbedderResult<T> = Result<T, EmbedderError>;

/// The core embedder trait.
///
/// Influenced by Graphiti's `EmbedderClient` pattern.
/// Separate from LLM because embedding models have different APIs
/// and can be local (FastEmbed) for zero-cost operation.
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Get the model name.
    fn model_name(&self) -> &str;

    /// Get the embedding dimension.
    fn dimensions(&self) -> usize;

    /// Embed a single text string.
    async fn embed_one(&self, text: &str) -> EmbedderResult<Vec<f32>> {
        let results = self.embed_batch(&[text.to_string()]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| EmbedderError::ApiError("Empty embedding result".into()))
    }

    /// Embed multiple texts in a batch (more efficient).
    async fn embed_batch(&self, texts: &[String]) -> EmbedderResult<Vec<Vec<f32>>>;
}
