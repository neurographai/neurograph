// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Embedder trait definition with rich model metadata and error taxonomy.
//!
//! The [`Embedder`] trait is the core abstraction for all embedding providers.
//! Implement this trait for any new provider that doesn't follow the
//! OpenAI-compatible `/v1/embeddings` format. For OpenAI-compatible APIs,
//! use [`super::openai_compatible::OpenAICompatibleEmbedder`] instead.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── Model Metadata ──────────────────────────────────────────────────────

/// Metadata about an embedding model — dimensions, cost, limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingModelInfo {
    /// Provider name: "openai", "gemini", "ollama", "local", "custom"
    pub provider: String,
    /// Model identifier: "text-embedding-3-large", "gemini-embedding-001"
    pub model_name: String,
    /// Output dimension (e.g., 1536, 3072, 768)
    pub dimensions: usize,
    /// Max input tokens the model accepts
    pub max_input_tokens: usize,
    /// Cost per 1M tokens in USD (0.0 for local models)
    pub cost_per_million_tokens: f64,
    /// Whether the model supports dimensionality reduction (Matryoshka)
    pub supports_shortening: bool,
    /// Whether outputs are already L2-normalized
    pub normalized_output: bool,
}

impl Default for EmbeddingModelInfo {
    fn default() -> Self {
        Self {
            provider: "hash".into(),
            model_name: "hash-embedder-v1".into(),
            dimensions: 384,
            max_input_tokens: usize::MAX,
            cost_per_million_tokens: 0.0,
            supports_shortening: false,
            normalized_output: true,
        }
    }
}

// ── Error Types ─────────────────────────────────────────────────────────

/// Errors from embedding operations.
#[derive(Debug, thiserror::Error)]
pub enum EmbedderError {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Model not available: {0}")]
    ModelNotAvailable(String),

    #[error("Rate limited — retry after {retry_after_ms}ms")]
    RateLimited {
        /// Suggested retry delay in milliseconds.
        retry_after_ms: u64,
    },

    #[error("Input too long: {tokens} tokens, max {max_tokens}")]
    InputTooLong {
        /// Actual token count.
        tokens: usize,
        /// Maximum allowed.
        max_tokens: usize,
    },

    #[error("Authentication failed — check API key")]
    AuthError,

    #[error("Provider unavailable: {0}")]
    Unavailable(String),

    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch {
        /// Expected dimension count.
        expected: usize,
        /// Actual dimension count.
        got: usize,
    },
}

pub type EmbedderResult<T> = Result<T, EmbedderError>;

// ── Core Trait ───────────────────────────────────────────────────────────

/// The core embedder trait.
///
/// Influenced by Graphiti's `EmbedderClient` pattern.
/// Separate from LLM because embedding models have different APIs
/// and can be local (hash, ONNX) for zero-cost operation.
///
/// # Implementors
///
/// - [`super::openai_compatible::OpenAICompatibleEmbedder`] — handles OpenAI,
///   Gemini, Cohere, Voyage, Jina, Mistral, Azure, Ollama, vLLM, LM Studio
/// - [`super::fastembed::HashEmbedder`] — deterministic hash embeddings
/// - Any custom type via `impl Embedder for MyType`
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Get the model name.
    fn model_name(&self) -> &str;

    /// Get the embedding dimension.
    fn dimensions(&self) -> usize;

    /// Rich model metadata (dimensions, cost, limits).
    ///
    /// Default implementation builds from `model_name()` and `dimensions()`.
    fn model_info(&self) -> EmbeddingModelInfo {
        EmbeddingModelInfo {
            provider: "unknown".into(),
            model_name: self.model_name().to_string(),
            dimensions: self.dimensions(),
            ..Default::default()
        }
    }

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

    /// Health check — can this provider serve requests right now?
    async fn health_check(&self) -> EmbedderResult<()> {
        self.embed_one("health check").await.map(|_| ())
    }
}
