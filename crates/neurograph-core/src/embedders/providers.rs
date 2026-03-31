// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Registry of known embedding providers and models.
//!
//! To add a new model, add an entry here — **no new client code needed**.
//! Every entry returns an [`OpenAICompatibleConfig`] that works with the
//! universal [`OpenAICompatibleEmbedder`].
//!
//! # Adding a New Provider
//!
//! ```rust,no_run
//! // In providers.rs — add a factory method:
//! pub fn new_provider_v1() -> OpenAICompatibleConfig { /* ... */ }
//!
//! // In the builder API — add a convenience method:
//! pub fn newprovider_embeddings(mut self) -> Self { /* ... */ }
//! ```

use super::openai_compatible::{ApiKeySource, OpenAICompatibleConfig};
use super::traits::EmbeddingModelInfo;
use std::collections::HashMap;

/// Registry of pre-configured embedding providers.
///
/// Every method returns an [`OpenAICompatibleConfig`] ready to pass
/// to [`OpenAICompatibleEmbedder::new()`].
pub struct EmbeddingRegistry;

impl EmbeddingRegistry {
    // ─────────────────── OpenAI ───────────────────

    /// OpenAI text-embedding-3-small (1536d, $0.02/1M tokens).
    pub fn openai_text_embedding_3_small() -> OpenAICompatibleConfig {
        OpenAICompatibleConfig {
            base_url: "https://api.openai.com/v1".into(),
            model: "text-embedding-3-small".into(),
            api_key: ApiKeySource::Env("OPENAI_API_KEY".into()),
            dimensions: None,
            encoding_format: None,
            timeout_secs: 30,
            max_batch_size: 2048,
            extra_headers: HashMap::new(),
            model_info: EmbeddingModelInfo {
                provider: "openai".into(),
                model_name: "text-embedding-3-small".into(),
                dimensions: 1536,
                max_input_tokens: 8191,
                cost_per_million_tokens: 0.02,
                supports_shortening: true,
                normalized_output: true,
            },
        }
    }

    /// OpenAI text-embedding-3-large (3072d, $0.13/1M tokens).
    pub fn openai_text_embedding_3_large() -> OpenAICompatibleConfig {
        let mut config = Self::openai_text_embedding_3_small();
        config.model = "text-embedding-3-large".into();
        config.model_info.model_name = "text-embedding-3-large".into();
        config.model_info.dimensions = 3072;
        config.model_info.cost_per_million_tokens = 0.13;
        config
    }

    /// OpenAI text-embedding-3-small with reduced dimensions (256d).
    pub fn openai_text_embedding_3_small_256d() -> OpenAICompatibleConfig {
        let mut config = Self::openai_text_embedding_3_small();
        config.dimensions = Some(256);
        config.model_info.dimensions = 256;
        config.model_info.model_name = "text-embedding-3-small-256d".into();
        config
    }

    // ─────────────────── Google Gemini ───────────────────

    /// Gemini text-embedding-004 (768d, free tier available).
    pub fn gemini_text_embedding_004() -> OpenAICompatibleConfig {
        OpenAICompatibleConfig {
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai".into(),
            model: "text-embedding-004".into(),
            api_key: ApiKeySource::Env("GEMINI_API_KEY".into()),
            dimensions: None,
            encoding_format: None,
            timeout_secs: 30,
            max_batch_size: 100,
            extra_headers: HashMap::new(),
            model_info: EmbeddingModelInfo {
                provider: "gemini".into(),
                model_name: "text-embedding-004".into(),
                dimensions: 768,
                max_input_tokens: 2048,
                cost_per_million_tokens: 0.0,
                supports_shortening: true,
                normalized_output: true,
            },
        }
    }

    /// Gemini embedding experimental (3072d, free).
    pub fn gemini_embedding_exp_03() -> OpenAICompatibleConfig {
        let mut config = Self::gemini_text_embedding_004();
        config.model = "gemini-embedding-exp-03".into();
        config.model_info.model_name = "gemini-embedding-exp-03".into();
        config.model_info.dimensions = 3072;
        config.model_info.max_input_tokens = 8192;
        config
    }

    /// Gemini Embedding 2 Preview (3072d, multimodal, MRL, $0.20/1M tokens).
    ///
    /// Released March 10, 2026. First multimodal embedding model in Gemini API.
    /// Supports text, image, video, audio, PDF. 8192 token context window.
    /// Native Matryoshka Representation Learning (MRL) — valid at 3072, 1536, 768.
    pub fn gemini_embedding_2_preview() -> OpenAICompatibleConfig {
        let mut config = Self::gemini_text_embedding_004();
        config.model = "gemini-embedding-2-preview".into();
        config.model_info.model_name = "gemini-embedding-2-preview".into();
        config.model_info.dimensions = 3072;
        config.model_info.max_input_tokens = 8192;
        config.model_info.cost_per_million_tokens = 0.20;
        config.model_info.supports_shortening = true; // MRL
        config
    }

    // ─────────────────── Cohere ───────────────────

    /// Cohere embed-v4.0 (1024d, $0.10/1M tokens).
    pub fn cohere_embed_v4() -> OpenAICompatibleConfig {
        OpenAICompatibleConfig {
            base_url: "https://api.cohere.com/compatibility/v1".into(),
            model: "embed-v4.0".into(),
            api_key: ApiKeySource::Env("COHERE_API_KEY".into()),
            dimensions: None,
            encoding_format: None,
            timeout_secs: 30,
            max_batch_size: 96,
            extra_headers: HashMap::new(),
            model_info: EmbeddingModelInfo {
                provider: "cohere".into(),
                model_name: "embed-v4.0".into(),
                dimensions: 1536, // v4 default is 1536, supports MRL down to 256
                max_input_tokens: 512,
                cost_per_million_tokens: 0.10,
                supports_shortening: false,
                normalized_output: true,
            },
        }
    }

    /// Cohere embed-english-v3.0 (1024d).
    pub fn cohere_embed_english_v3() -> OpenAICompatibleConfig {
        let mut config = Self::cohere_embed_v4();
        config.model = "embed-english-v3.0".into();
        config.model_info.model_name = "embed-english-v3.0".into();
        config
    }

    // ─────────────────── Voyage AI ───────────────────

    /// Voyage voyage-3-large (1024d, $0.18/1M tokens).
    pub fn voyage_3_large() -> OpenAICompatibleConfig {
        OpenAICompatibleConfig {
            base_url: "https://api.voyageai.com/v1".into(),
            model: "voyage-3-large".into(),
            api_key: ApiKeySource::Env("VOYAGE_API_KEY".into()),
            dimensions: None,
            encoding_format: None,
            timeout_secs: 30,
            max_batch_size: 128,
            extra_headers: HashMap::new(),
            model_info: EmbeddingModelInfo {
                provider: "voyage".into(),
                model_name: "voyage-3-large".into(),
                dimensions: 1024,
                max_input_tokens: 32000,
                cost_per_million_tokens: 0.18,
                supports_shortening: false,
                normalized_output: true,
            },
        }
    }

    /// Voyage voyage-code-3 (1024d, optimized for code).
    pub fn voyage_code_3() -> OpenAICompatibleConfig {
        let mut config = Self::voyage_3_large();
        config.model = "voyage-code-3".into();
        config.model_info.model_name = "voyage-code-3".into();
        config.model_info.max_input_tokens = 16000;
        config
    }

    /// Voyage 4 Large (1024d, MoE architecture, $0.18/1M tokens).
    ///
    /// MongoDB-Voyage AI partnership, Jan 2026. First production MoE
    /// embedding model. OpenAI-compatible endpoint.
    pub fn voyage_4_large() -> OpenAICompatibleConfig {
        let mut config = Self::voyage_3_large();
        config.model = "voyage-4-large".into();
        config.model_info.model_name = "voyage-4-large".into();
        config.model_info.max_input_tokens = 32000;
        config
    }

    /// Voyage 4 Lite (512d, lightweight, $0.05/1M tokens).
    pub fn voyage_4_lite() -> OpenAICompatibleConfig {
        let mut config = Self::voyage_3_large();
        config.model = "voyage-4-lite".into();
        config.model_info.model_name = "voyage-4-lite".into();
        config.model_info.dimensions = 512;
        config.model_info.cost_per_million_tokens = 0.05;
        config
    }

    // ─────────────────── Jina AI ───────────────────

    /// Jina jina-embeddings-v3 (1024d, $0.02/1M tokens).
    pub fn jina_embeddings_v3() -> OpenAICompatibleConfig {
        OpenAICompatibleConfig {
            base_url: "https://api.jina.ai/v1".into(),
            model: "jina-embeddings-v3".into(),
            api_key: ApiKeySource::Env("JINA_API_KEY".into()),
            dimensions: Some(1024),
            encoding_format: None,
            timeout_secs: 30,
            max_batch_size: 500,
            extra_headers: HashMap::new(),
            model_info: EmbeddingModelInfo {
                provider: "jina".into(),
                model_name: "jina-embeddings-v3".into(),
                dimensions: 1024,
                max_input_tokens: 8192,
                cost_per_million_tokens: 0.018,
                supports_shortening: true,
                normalized_output: true,
            },
        }
    }

    /// Jina jina-embeddings-v4 (2048d, multimodal, $0.05/1M tokens).
    ///
    /// SOTA on ViDoRe and multimodal benchmarks. Supports 32K context.
    /// OpenAI-compatible endpoint.
    pub fn jina_embeddings_v4() -> OpenAICompatibleConfig {
        let mut config = Self::jina_embeddings_v3();
        config.model = "jina-embeddings-v4".into();
        config.dimensions = Some(2048);
        config.model_info.model_name = "jina-embeddings-v4".into();
        config.model_info.dimensions = 2048;
        config.model_info.max_input_tokens = 32768;
        config.model_info.cost_per_million_tokens = 0.05;
        config
    }

    // ─────────────────── Mistral ───────────────────

    /// Mistral mistral-embed (1024d, $0.10/1M tokens).
    pub fn mistral_embed() -> OpenAICompatibleConfig {
        OpenAICompatibleConfig {
            base_url: "https://api.mistral.ai/v1".into(),
            model: "mistral-embed".into(),
            api_key: ApiKeySource::Env("MISTRAL_API_KEY".into()),
            dimensions: None,
            encoding_format: None,
            timeout_secs: 30,
            max_batch_size: 512,
            extra_headers: HashMap::new(),
            model_info: EmbeddingModelInfo {
                provider: "mistral".into(),
                model_name: "mistral-embed".into(),
                dimensions: 1024,
                max_input_tokens: 8192,
                cost_per_million_tokens: 0.10,
                supports_shortening: false,
                normalized_output: true,
            },
        }
    }

    // ─────────────────── Azure OpenAI ───────────────────

    /// Azure OpenAI embeddings (configure per deployment).
    pub fn azure_openai(
        resource_name: &str,
        deployment: &str,
        api_version: &str,
    ) -> OpenAICompatibleConfig {
        let mut headers = HashMap::new();
        headers.insert("api-version".into(), api_version.into());

        OpenAICompatibleConfig {
            base_url: format!(
                "https://{}.openai.azure.com/openai/deployments/{}",
                resource_name, deployment
            ),
            model: deployment.into(),
            api_key: ApiKeySource::Env("AZURE_OPENAI_API_KEY".into()),
            dimensions: None,
            encoding_format: None,
            timeout_secs: 30,
            max_batch_size: 2048,
            extra_headers: headers,
            model_info: EmbeddingModelInfo {
                provider: "azure".into(),
                model_name: deployment.into(),
                dimensions: 1536, // depends on deployed model
                max_input_tokens: 8191,
                cost_per_million_tokens: 0.02,
                supports_shortening: false,
                normalized_output: true,
            },
        }
    }

    // ─────────────────── Ollama (Local, via /v1 compat) ───────────────────

    /// Ollama local embeddings via OpenAI-compatible endpoint.
    pub fn ollama(model: &str) -> OpenAICompatibleConfig {
        let (dims, max_tokens) = Self::ollama_model_info(model);
        let base_host = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://localhost:11434".into());

        OpenAICompatibleConfig {
            base_url: format!("{}/v1", base_host.trim_end_matches('/')),
            model: model.into(),
            api_key: ApiKeySource::None,
            dimensions: None,
            encoding_format: None,
            timeout_secs: 60, // local models can be slower
            max_batch_size: 512,
            extra_headers: HashMap::new(),
            model_info: EmbeddingModelInfo {
                provider: "ollama".into(),
                model_name: model.into(),
                dimensions: dims,
                max_input_tokens: max_tokens,
                cost_per_million_tokens: 0.0,
                supports_shortening: false,
                normalized_output: false,
            },
        }
    }

    /// Known Ollama embedding model dimensions.
    fn ollama_model_info(model: &str) -> (usize, usize) {
        match model {
            "nomic-embed-text" => (768, 8192),
            "mxbai-embed-large" => (1024, 512),
            "snowflake-arctic-embed" | "snowflake-arctic-embed:m" => (1024, 512),
            "all-minilm" | "all-minilm:l6-v2" => (384, 512),
            "bge-large" | "bge-large:en-v1.5" => (1024, 512),
            "bge-m3" => (1024, 8192),
            _ => (768, 512), // safe default
        }
    }

    // ─────────────────── vLLM / LM Studio / LocalAI ───────────────────

    /// Any local OpenAI-compatible embedding server.
    pub fn local_server(base_url: &str, model: &str, dimensions: usize) -> OpenAICompatibleConfig {
        OpenAICompatibleConfig {
            base_url: base_url.into(),
            model: model.into(),
            api_key: ApiKeySource::None,
            dimensions: None,
            encoding_format: None,
            timeout_secs: 60,
            max_batch_size: 128,
            extra_headers: HashMap::new(),
            model_info: EmbeddingModelInfo {
                provider: "local".into(),
                model_name: model.into(),
                dimensions,
                max_input_tokens: 8192,
                cost_per_million_tokens: 0.0,
                supports_shortening: false,
                normalized_output: false,
            },
        }
    }

    // ─────────────────── Qwen3 (via Ollama) ───────────────────

    /// Qwen3-Embedding-0.6B via Ollama (768d, free, local).
    ///
    /// Latest from Qwen team, optimized for semantic search, reranking,
    /// clustering, and classification. Run via Ollama.
    pub fn qwen3_embedding() -> OpenAICompatibleConfig {
        Self::ollama("qwen3-embedding:0.6b")
    }

    // ─────────────────── Future-Proofing ───────────────────

    /// Create config for ANY provider with an OpenAI-compatible endpoint.
    ///
    /// This is the escape hatch for providers that ship next month.
    pub fn custom_openai_compatible(
        base_url: &str,
        model: &str,
        api_key_env: &str,
        dimensions: usize,
    ) -> OpenAICompatibleConfig {
        OpenAICompatibleConfig {
            base_url: base_url.into(),
            model: model.into(),
            api_key: if api_key_env.is_empty() {
                ApiKeySource::None
            } else {
                ApiKeySource::Env(api_key_env.into())
            },
            dimensions: None,
            encoding_format: None,
            timeout_secs: 30,
            max_batch_size: 100,
            extra_headers: HashMap::new(),
            model_info: EmbeddingModelInfo {
                provider: "custom".into(),
                model_name: model.into(),
                dimensions,
                max_input_tokens: 8192,
                cost_per_million_tokens: 0.0,
                supports_shortening: false,
                normalized_output: false,
            },
        }
    }

    /// List all known providers and models.
    ///
    /// Returns `(provider, model, dimensions, cost_per_million_tokens)`.
    pub fn list_all() -> Vec<(&'static str, &'static str, usize, f64)> {
        vec![
            // OpenAI
            ("openai", "text-embedding-3-small", 1536, 0.02),
            ("openai", "text-embedding-3-large", 3072, 0.13),
            // Gemini
            ("gemini", "text-embedding-004", 768, 0.00),
            ("gemini", "gemini-embedding-exp-03", 3072, 0.00),
            ("gemini", "gemini-embedding-2-preview", 3072, 0.20),
            // Cohere
            ("cohere", "embed-v4.0", 1536, 0.10),
            ("cohere", "embed-english-v3.0", 1024, 0.10),
            // Voyage AI
            ("voyage", "voyage-3-large", 1024, 0.18),
            ("voyage", "voyage-4-large", 1024, 0.18),
            ("voyage", "voyage-4-lite", 512, 0.05),
            ("voyage", "voyage-code-3", 1024, 0.18),
            // Jina AI
            ("jina", "jina-embeddings-v3", 1024, 0.018),
            ("jina", "jina-embeddings-v4", 2048, 0.05),
            // Mistral
            ("mistral", "mistral-embed", 1024, 0.10),
            // Ollama (local)
            ("ollama", "nomic-embed-text", 768, 0.00),
            ("ollama", "mxbai-embed-large", 1024, 0.00),
            ("ollama", "snowflake-arctic-embed", 1024, 0.00),
            ("ollama", "bge-m3", 1024, 0.00),
            ("ollama", "qwen3-embedding:0.6b", 768, 0.00),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_config() {
        let config = EmbeddingRegistry::openai_text_embedding_3_small();
        assert_eq!(config.base_url, "https://api.openai.com/v1");
        assert_eq!(config.model, "text-embedding-3-small");
        assert_eq!(config.model_info.dimensions, 1536);
        assert!(matches!(config.api_key, ApiKeySource::Env(ref k) if k == "OPENAI_API_KEY"));
    }

    #[test]
    fn test_gemini_config() {
        let config = EmbeddingRegistry::gemini_text_embedding_004();
        assert!(config.base_url.contains("generativelanguage.googleapis.com"));
        assert_eq!(config.model_info.dimensions, 768);
        assert_eq!(config.model_info.cost_per_million_tokens, 0.0);
    }

    #[test]
    fn test_ollama_config() {
        let config = EmbeddingRegistry::ollama("nomic-embed-text");
        assert!(config.base_url.contains("/v1"));
        assert_eq!(config.model_info.dimensions, 768);
        assert!(matches!(config.api_key, ApiKeySource::None));
    }

    #[test]
    fn test_custom_config() {
        let config = EmbeddingRegistry::custom_openai_compatible(
            "https://api.newco.com/v1",
            "embed-v1",
            "NEWCO_KEY",
            2048,
        );
        assert_eq!(config.base_url, "https://api.newco.com/v1");
        assert_eq!(config.model_info.dimensions, 2048);
    }

    #[test]
    fn test_list_all() {
        let providers = EmbeddingRegistry::list_all();
        assert!(providers.len() >= 14);
        assert!(providers.iter().any(|p| p.0 == "openai"));
        assert!(providers.iter().any(|p| p.0 == "gemini"));
        assert!(providers.iter().any(|p| p.0 == "ollama"));
    }

    #[test]
    fn test_azure_config() {
        let config = EmbeddingRegistry::azure_openai("myresource", "my-deploy", "2024-02-01");
        assert!(config.base_url.contains("myresource"));
        assert!(config.extra_headers.contains_key("api-version"));
    }
}
