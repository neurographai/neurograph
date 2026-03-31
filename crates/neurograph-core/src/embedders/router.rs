// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Embedding model router with provider selection, factory, and fallback chains.
//!
//! # Architecture
//!
//! ```text
//! EmbeddingConfig  ──▶  EmbeddingFactory::build()  ──▶  Arc<dyn Embedder>
//!                                                          │
//!                                                   EmbeddingRouter
//!                                                   (primary + fallbacks)
//! ```
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use neurograph_core::embedders::router::{EmbeddingConfig, EmbeddingFactory};
//!
//! let config = EmbeddingConfig::Gemini;
//! let embedder = EmbeddingFactory::build(&config).unwrap();
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::cache::EmbeddingCache;
use super::fastembed::HashEmbedder;
use super::ollama::OllamaEmbedder;
use super::openai_compatible::{OpenAICompatibleConfig, OpenAICompatibleEmbedder};
use super::providers::EmbeddingRegistry;
use super::traits::{Embedder, EmbedderError, EmbedderResult, EmbeddingModelInfo};

// ── EmbeddingConfig ─────────────────────────────────────────────────────

/// High-level embedding configuration for the builder API.
///
/// Each variant maps to a pre-configured provider. Use `Custom` for
/// any OpenAI-compatible endpoint not listed here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingConfig {
    // ── Pre-configured API providers ──
    /// OpenAI text-embedding-3-small (default API provider).
    OpenAI,
    /// OpenAI with specific model name.
    OpenAIModel(String),
    /// Google Gemini text-embedding-004.
    Gemini,
    /// Gemini with specific model name.
    GeminiModel(String),
    /// Cohere embed-v4.0.
    Cohere,
    /// Voyage AI voyage-3-large.
    Voyage,
    /// Jina AI jina-embeddings-v3.
    Jina,
    /// Mistral mistral-embed.
    Mistral,

    // ── Local providers ──
    /// Ollama with model name (e.g., "nomic-embed-text").
    Ollama(String),

    // ── Zero-cost ──
    /// Hash-based embeddings (no API, no model, deterministic).
    Hash {
        /// Output dimensions.
        dimensions: usize,
    },

    // ── Escape hatch ──
    /// Any OpenAI-compatible endpoint.
    Custom(OpenAICompatibleConfig),
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self::Hash { dimensions: 384 }
    }
}

// ── EmbeddingFactory ────────────────────────────────────────────────────

/// Builds the appropriate embedder from configuration.
pub struct EmbeddingFactory;

impl EmbeddingFactory {
    /// Create an embedder from config.
    ///
    /// Returns `Arc<dyn Embedder>` ready for injection into NeuroGraph.
    pub fn build(config: &EmbeddingConfig) -> Result<Arc<dyn Embedder>, EmbedderError> {
        let embedder: Arc<dyn Embedder> = match config {
            // Pre-configured API providers
            EmbeddingConfig::OpenAI => Arc::new(OpenAICompatibleEmbedder::new(
                EmbeddingRegistry::openai_text_embedding_3_small(),
            )?),

            EmbeddingConfig::OpenAIModel(model) => {
                let cfg = match model.as_str() {
                    "text-embedding-3-small" => {
                        EmbeddingRegistry::openai_text_embedding_3_small()
                    }
                    "text-embedding-3-large" => {
                        EmbeddingRegistry::openai_text_embedding_3_large()
                    }
                    _ => EmbeddingRegistry::custom_openai_compatible(
                        "https://api.openai.com/v1",
                        model,
                        "OPENAI_API_KEY",
                        1536,
                    ),
                };
                Arc::new(OpenAICompatibleEmbedder::new(cfg)?)
            }

            EmbeddingConfig::Gemini => Arc::new(OpenAICompatibleEmbedder::new(
                EmbeddingRegistry::gemini_text_embedding_004(),
            )?),

            EmbeddingConfig::GeminiModel(model) => {
                let cfg = match model.as_str() {
                    "text-embedding-004" => EmbeddingRegistry::gemini_text_embedding_004(),
                    "gemini-embedding-exp-03" => EmbeddingRegistry::gemini_embedding_exp_03(),
                    _ => EmbeddingRegistry::custom_openai_compatible(
                        "https://generativelanguage.googleapis.com/v1beta/openai",
                        model,
                        "GEMINI_API_KEY",
                        768,
                    ),
                };
                Arc::new(OpenAICompatibleEmbedder::new(cfg)?)
            }

            EmbeddingConfig::Cohere => Arc::new(OpenAICompatibleEmbedder::new(
                EmbeddingRegistry::cohere_embed_v4(),
            )?),

            EmbeddingConfig::Voyage => Arc::new(OpenAICompatibleEmbedder::new(
                EmbeddingRegistry::voyage_3_large(),
            )?),

            EmbeddingConfig::Jina => Arc::new(OpenAICompatibleEmbedder::new(
                EmbeddingRegistry::jina_embeddings_v3(),
            )?),

            EmbeddingConfig::Mistral => Arc::new(OpenAICompatibleEmbedder::new(
                EmbeddingRegistry::mistral_embed(),
            )?),

            // Local providers
            EmbeddingConfig::Ollama(model) => Arc::new(OpenAICompatibleEmbedder::new(
                EmbeddingRegistry::ollama(model),
            )?),

            // Hash (zero-cost)
            EmbeddingConfig::Hash { dimensions } => Arc::new(HashEmbedder::new(*dimensions)),

            // Custom OpenAI-compatible
            EmbeddingConfig::Custom(cfg) => {
                Arc::new(OpenAICompatibleEmbedder::new(cfg.clone())?)
            }
        };

        Ok(embedder)
    }
}

// ── EmbeddingRouter ─────────────────────────────────────────────────────

/// Primary embedder with fallback chain.
///
/// Tries the primary embedder first; on failure, tries each fallback
/// in order. This enables graceful degradation when API providers
/// are temporarily unavailable.
pub struct EmbeddingRouter {
    primary: Arc<dyn Embedder>,
    fallbacks: Vec<Arc<dyn Embedder>>,
}

impl std::fmt::Debug for EmbeddingRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddingRouter")
            .field("primary", &self.primary.model_name())
            .field("fallback_count", &self.fallbacks.len())
            .finish()
    }
}

impl EmbeddingRouter {
    /// Create a router with a primary embedder.
    pub fn new(primary: Arc<dyn Embedder>) -> Self {
        Self {
            primary,
            fallbacks: vec![],
        }
    }

    /// Add a fallback embedder (tried in order if primary fails).
    pub fn with_fallback(mut self, fallback: Arc<dyn Embedder>) -> Self {
        self.fallbacks.push(fallback);
        self
    }
}

#[async_trait]
impl Embedder for EmbeddingRouter {
    fn model_name(&self) -> &str {
        self.primary.model_name()
    }

    fn dimensions(&self) -> usize {
        self.primary.dimensions()
    }

    fn model_info(&self) -> EmbeddingModelInfo {
        self.primary.model_info()
    }

    async fn embed_one(&self, text: &str) -> EmbedderResult<Vec<f32>> {
        // Try primary
        match self.primary.embed_one(text).await {
            Ok(v) => return Ok(v),
            Err(e) => {
                tracing::warn!(
                    primary = self.primary.model_name(),
                    error = %e,
                    "Primary embedder failed, trying fallbacks"
                );
            }
        }

        // Try fallbacks in order
        for (i, fallback) in self.fallbacks.iter().enumerate() {
            match fallback.embed_one(text).await {
                Ok(v) => {
                    tracing::info!(
                        fallback_index = i,
                        model = fallback.model_name(),
                        "Fallback embedder succeeded"
                    );
                    return Ok(v);
                }
                Err(e) => {
                    tracing::warn!(
                        fallback_index = i,
                        error = %e,
                        "Fallback embedder failed"
                    );
                }
            }
        }

        Err(EmbedderError::Unavailable(
            "All embedding providers failed".into(),
        ))
    }

    async fn embed_batch(&self, texts: &[String]) -> EmbedderResult<Vec<Vec<f32>>> {
        // Try primary
        match self.primary.embed_batch(texts).await {
            Ok(v) => return Ok(v),
            Err(e) => {
                tracing::warn!(
                    primary = self.primary.model_name(),
                    error = %e,
                    "Primary embedder batch failed, trying fallbacks"
                );
            }
        }

        // Try fallbacks
        for (i, fallback) in self.fallbacks.iter().enumerate() {
            match fallback.embed_batch(texts).await {
                Ok(v) => {
                    tracing::info!(fallback_index = i, "Fallback batch succeeded");
                    return Ok(v);
                }
                Err(e) => {
                    tracing::warn!(fallback_index = i, error = %e, "Fallback batch failed");
                }
            }
        }

        Err(EmbedderError::Unavailable(
            "All embedding providers failed".into(),
        ))
    }
}

// ── Legacy Compat ───────────────────────────────────────────────────────

/// Parsed embedding specification (legacy format).
#[derive(Debug, Clone)]
pub struct EmbedSpec {
    pub provider: String,
    pub model: String,
}

/// Parse a "provider:model" string into components (legacy compat).
pub fn parse_spec(spec: &str) -> EmbedSpec {
    match spec.split_once(':') {
        Some((provider, model)) => EmbedSpec {
            provider: provider.to_lowercase(),
            model: model.to_string(),
        },
        None => EmbedSpec {
            provider: spec.to_lowercase(),
            model: "default".to_string(),
        },
    }
}

/// Convert a legacy spec string into an EmbeddingConfig.
pub fn spec_to_config(spec: &str) -> EmbeddingConfig {
    let parsed = parse_spec(spec);
    match parsed.provider.as_str() {
        "hash" => EmbeddingConfig::Hash { dimensions: 384 },
        "openai" => EmbeddingConfig::OpenAIModel(
            if parsed.model == "default" {
                "text-embedding-3-small".into()
            } else {
                parsed.model
            },
        ),
        "gemini" => EmbeddingConfig::GeminiModel(
            if parsed.model == "default" {
                "text-embedding-004".into()
            } else {
                parsed.model
            },
        ),
        "cohere" => EmbeddingConfig::Cohere,
        "voyage" => EmbeddingConfig::Voyage,
        "jina" => EmbeddingConfig::Jina,
        "mistral" => EmbeddingConfig::Mistral,
        "ollama" => EmbeddingConfig::Ollama(
            if parsed.model == "default" {
                "nomic-embed-text".into()
            } else {
                parsed.model
            },
        ),
        _ => EmbeddingConfig::Hash { dimensions: 384 },
    }
}

/// Create an Ollama embedder from a model name (legacy compat).
pub fn create_ollama_embedder(model: &str) -> Result<OllamaEmbedder, EmbedderError> {
    OllamaEmbedder::new(model)
}

/// Convenience: create an Ollama embedder with a cache (legacy compat).
pub fn create_cached_ollama(
    model: &str,
    cache_capacity: usize,
) -> Result<(OllamaEmbedder, Arc<EmbeddingCache>), EmbedderError> {
    let embedder = OllamaEmbedder::new(model)?;
    let cache = Arc::new(EmbeddingCache::new(cache_capacity));
    Ok((embedder, cache))
}

/// Print available embedding providers (expanded).
pub fn available_providers() -> Vec<(&'static str, &'static str, bool)> {
    vec![
        (
            "hash",
            "Deterministic hash embeddings (always available)",
            true,
        ),
        (
            "openai",
            "OpenAI text-embedding-3-small/large (requires OPENAI_API_KEY)",
            std::env::var("OPENAI_API_KEY").is_ok(),
        ),
        (
            "gemini",
            "Google Gemini text-embedding-004 (requires GEMINI_API_KEY, free tier)",
            std::env::var("GEMINI_API_KEY").is_ok(),
        ),
        (
            "cohere",
            "Cohere embed-v4.0 (requires COHERE_API_KEY)",
            std::env::var("COHERE_API_KEY").is_ok(),
        ),
        (
            "voyage",
            "Voyage AI voyage-3-large (requires VOYAGE_API_KEY)",
            std::env::var("VOYAGE_API_KEY").is_ok(),
        ),
        (
            "jina",
            "Jina AI jina-embeddings-v3 (requires JINA_API_KEY)",
            std::env::var("JINA_API_KEY").is_ok(),
        ),
        (
            "mistral",
            "Mistral mistral-embed (requires MISTRAL_API_KEY)",
            std::env::var("MISTRAL_API_KEY").is_ok(),
        ),
        (
            "ollama",
            "Ollama local embeddings (requires running Ollama server)",
            true,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_spec_with_model() {
        let spec = parse_spec("ollama:nomic-embed-text");
        assert_eq!(spec.provider, "ollama");
        assert_eq!(spec.model, "nomic-embed-text");
    }

    #[test]
    fn test_parse_spec_without_model() {
        let spec = parse_spec("hash");
        assert_eq!(spec.provider, "hash");
        assert_eq!(spec.model, "default");
    }

    #[test]
    fn test_parse_spec_openai() {
        let spec = parse_spec("openai:text-embedding-3-small");
        assert_eq!(spec.provider, "openai");
        assert_eq!(spec.model, "text-embedding-3-small");
    }

    #[test]
    fn test_available_providers() {
        let providers = available_providers();
        assert!(providers.len() >= 8);
    }

    #[test]
    fn test_spec_to_config() {
        let config = spec_to_config("openai:text-embedding-3-large");
        assert!(matches!(config, EmbeddingConfig::OpenAIModel(ref m) if m == "text-embedding-3-large"));

        let config = spec_to_config("gemini");
        assert!(matches!(config, EmbeddingConfig::GeminiModel(_)));

        let config = spec_to_config("hash");
        assert!(matches!(config, EmbeddingConfig::Hash { .. }));
    }

    #[test]
    fn test_factory_build_hash() {
        let config = EmbeddingConfig::Hash { dimensions: 256 };
        let embedder = EmbeddingFactory::build(&config).unwrap();
        assert_eq!(embedder.dimensions(), 256);
    }

    #[test]
    fn test_embedding_config_default() {
        let config = EmbeddingConfig::default();
        assert!(matches!(config, EmbeddingConfig::Hash { dimensions: 384 }));
    }
}
