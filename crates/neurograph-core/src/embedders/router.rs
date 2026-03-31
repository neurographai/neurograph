// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Embedding model router — parse "provider:model" specs.
//!
//! # Format
//! - `hash` or `hash:default` → deterministic hash embeddings
//! - `openai:text-embedding-3-small` → OpenAI embeddings
//! - `ollama:nomic-embed-text` → Ollama local embeddings

use super::cache::EmbeddingCache;
use super::ollama::OllamaEmbedder;

/// Parsed embedding specification.
#[derive(Debug, Clone)]
pub struct EmbedSpec {
    pub provider: String,
    pub model: String,
}

/// Parse a "provider:model" string into components.
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

/// Create an Ollama embedder from a model name.
pub fn create_ollama_embedder(model: &str) -> Result<OllamaEmbedder, super::traits::EmbedderError> {
    OllamaEmbedder::new(model)
}

/// Convenience: create an Ollama embedder with a cache.
pub fn create_cached_ollama(
    model: &str,
    cache_capacity: usize,
) -> Result<(OllamaEmbedder, std::sync::Arc<EmbeddingCache>), super::traits::EmbedderError> {
    let embedder = OllamaEmbedder::new(model)?;
    let cache = std::sync::Arc::new(EmbeddingCache::new(cache_capacity));
    Ok((embedder, cache))
}

/// Print available embedding providers.
pub fn available_providers() -> Vec<(&'static str, &'static str, bool)> {
    vec![
        ("hash", "Deterministic hash embeddings (always available)", true),
        (
            "openai",
            "OpenAI text-embedding-3-small/large (requires OPENAI_API_KEY)",
            std::env::var("OPENAI_API_KEY").is_ok(),
        ),
        (
            "ollama",
            "Ollama local embeddings (requires running Ollama server)",
            true, // can't check without async
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
        assert!(providers.len() >= 3);
    }
}
