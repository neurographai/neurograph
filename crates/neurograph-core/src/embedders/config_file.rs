// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! TOML-based embedding provider configuration.
//!
//! Enables defining embedding providers in a config file so that
//! new models can be added with **zero code changes**.
//!
//! # Example
//!
//! ```toml
//! [embeddings]
//! active = "gemini-2"
//!
//! [embeddings.providers.openai-large]
//! type = "openai-compat"
//! base_url = "https://api.openai.com/v1"
//! api_key_env = "OPENAI_API_KEY"
//! model = "text-embedding-3-large"
//! dimensions = 3072
//! max_input_tokens = 8191
//! cost_per_million_tokens = 0.13
//!
//! [embeddings.providers.gemini-2]
//! type = "gemini"
//! api_key_env = "GEMINI_API_KEY"
//! model = "gemini-embedding-2-preview"
//! dimensions = 3072
//!
//! # Future model — zero code changes!
//! [embeddings.providers.future-model]
//! type = "openai-compat"
//! base_url = "https://api.futureai.com/v1"
//! api_key_env = "FUTUREAI_API_KEY"
//! model = "future-embed-v1"
//! dimensions = 4096
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! use neurograph_core::embedders::config_file::build_from_toml;
//!
//! let embedder = build_from_toml("neurograph.toml").unwrap();
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use super::fastembed::HashEmbedder;
use super::openai_compatible::{ApiKeySource, OpenAICompatibleConfig, OpenAICompatibleEmbedder};
use super::providers::EmbeddingRegistry;
use super::router::EmbeddingRouter;
use super::traits::{Embedder, EmbedderError, EmbeddingModelInfo};

// ── TOML Schema ─────────────────────────────────────────────────────────

/// Top-level config file structure.
///
/// Expects a `[embeddings]` table with `active` key and `[embeddings.providers.*]`
/// sub-tables.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub embeddings: EmbeddingsFileConfig,
}

/// The `[embeddings]` section of the config file.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmbeddingsFileConfig {
    /// Alias of the provider to use as default.
    #[serde(default = "default_active")]
    pub active: String,

    /// Map of alias → provider config.
    #[serde(default)]
    pub providers: HashMap<String, ProviderEntry>,
}

fn default_active() -> String {
    "hash".into()
}

impl Default for EmbeddingsFileConfig {
    fn default() -> Self {
        Self {
            active: default_active(),
            providers: HashMap::new(),
        }
    }
}

/// Configuration for a single embedding provider.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderEntry {
    /// Provider type: "openai-compat", "gemini", "cohere", "ollama", "hash",
    /// or any future type (unknown types fall back to "openai-compat").
    #[serde(rename = "type")]
    pub provider_type: String,

    /// API base URL.
    #[serde(default)]
    pub base_url: Option<String>,

    /// Model identifier.
    #[serde(default)]
    pub model: Option<String>,

    /// Environment variable name for the API key.
    #[serde(default)]
    pub api_key_env: Option<String>,

    /// Direct API key value (not recommended for production).
    #[serde(default)]
    pub api_key: Option<String>,

    /// Output embedding dimensions.
    #[serde(default = "default_dimensions")]
    pub dimensions: usize,

    /// Maximum input tokens.
    #[serde(default = "default_max_tokens")]
    pub max_input_tokens: usize,

    /// Cost per million tokens in USD.
    #[serde(default)]
    pub cost_per_million_tokens: f64,

    /// Whether the model supports Matryoshka/dimension reduction.
    #[serde(default)]
    pub supports_mrl: bool,

    /// Whether the model runs locally.
    #[serde(default)]
    pub is_local: bool,

    /// Request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Max batch size.
    #[serde(default = "default_batch")]
    pub max_batch_size: usize,

    /// Extra HTTP headers.
    #[serde(default)]
    pub extra_headers: HashMap<String, String>,
}

fn default_dimensions() -> usize {
    768
}
fn default_max_tokens() -> usize {
    8192
}
fn default_timeout() -> u64 {
    30
}
fn default_batch() -> usize {
    100
}

// ── Builder Functions ───────────────────────────────────────────────────

/// Build an `EmbeddingRouter` from a TOML config file path.
///
/// The router has the `active` provider set, plus a hash fallback.
///
/// # Errors
///
/// Returns `EmbedderError::ConfigError` if:
/// - The file cannot be read
/// - The TOML is malformed
/// - The `active` provider doesn't exist in the providers map
pub fn build_from_toml(path: impl AsRef<Path>) -> Result<Arc<dyn Embedder>, EmbedderError> {
    let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
        EmbedderError::ConfigError(format!("Failed to read config file: {}", e))
    })?;

    build_from_toml_str(&content)
}

/// Build an `EmbeddingRouter` from a TOML string.
pub fn build_from_toml_str(toml_str: &str) -> Result<Arc<dyn Embedder>, EmbedderError> {
    let config: ConfigFile = toml::from_str(toml_str).map_err(|e| {
        EmbedderError::ConfigError(format!("Invalid TOML config: {}", e))
    })?;

    build_from_config(&config.embeddings)
}

/// Build an `EmbeddingRouter` from a parsed `EmbeddingsFileConfig`.
pub fn build_from_config(config: &EmbeddingsFileConfig) -> Result<Arc<dyn Embedder>, EmbedderError> {
    // Build the active provider
    let active_entry = config.providers.get(&config.active).ok_or_else(|| {
        EmbedderError::ConfigError(format!(
            "Active provider '{}' not found. Available: {:?}",
            config.active,
            config.providers.keys().collect::<Vec<_>>()
        ))
    })?;

    let primary = build_provider(&config.active, active_entry)?;

    // Build router with hash fallback
    let router = EmbeddingRouter::new(primary)
        .with_fallback(Arc::new(HashEmbedder::new(active_entry.dimensions)));

    Ok(Arc::new(router))
}

/// Build a single embedder from a `ProviderEntry`.
fn build_provider(
    alias: &str,
    entry: &ProviderEntry,
) -> Result<Arc<dyn Embedder>, EmbedderError> {
    match entry.provider_type.as_str() {
        "hash" => Ok(Arc::new(HashEmbedder::new(entry.dimensions))),

        // Known pre-configured providers (use registry defaults)
        "openai" => {
            let model = entry.model.as_deref().unwrap_or("text-embedding-3-small");
            let cfg = match model {
                "text-embedding-3-small" => EmbeddingRegistry::openai_text_embedding_3_small(),
                "text-embedding-3-large" => EmbeddingRegistry::openai_text_embedding_3_large(),
                _ => EmbeddingRegistry::custom_openai_compatible(
                    "https://api.openai.com/v1",
                    model,
                    entry.api_key_env.as_deref().unwrap_or("OPENAI_API_KEY"),
                    entry.dimensions,
                ),
            };
            Ok(Arc::new(OpenAICompatibleEmbedder::new(cfg)?))
        }

        "gemini" => {
            let model = entry.model.as_deref().unwrap_or("text-embedding-004");
            let cfg = match model {
                "text-embedding-004" => EmbeddingRegistry::gemini_text_embedding_004(),
                "gemini-embedding-exp-03" => EmbeddingRegistry::gemini_embedding_exp_03(),
                "gemini-embedding-2-preview" => EmbeddingRegistry::gemini_embedding_2_preview(),
                _ => EmbeddingRegistry::custom_openai_compatible(
                    "https://generativelanguage.googleapis.com/v1beta/openai",
                    model,
                    entry.api_key_env.as_deref().unwrap_or("GEMINI_API_KEY"),
                    entry.dimensions,
                ),
            };
            Ok(Arc::new(OpenAICompatibleEmbedder::new(cfg)?))
        }

        "cohere" => {
            let cfg = EmbeddingRegistry::cohere_embed_v4();
            Ok(Arc::new(OpenAICompatibleEmbedder::new(cfg)?))
        }

        "voyage" => {
            let model = entry.model.as_deref().unwrap_or("voyage-3-large");
            let cfg = match model {
                "voyage-3-large" => EmbeddingRegistry::voyage_3_large(),
                "voyage-4-large" => EmbeddingRegistry::voyage_4_large(),
                "voyage-4-lite" => EmbeddingRegistry::voyage_4_lite(),
                "voyage-code-3" => EmbeddingRegistry::voyage_code_3(),
                _ => EmbeddingRegistry::custom_openai_compatible(
                    "https://api.voyageai.com/v1",
                    model,
                    entry.api_key_env.as_deref().unwrap_or("VOYAGE_API_KEY"),
                    entry.dimensions,
                ),
            };
            Ok(Arc::new(OpenAICompatibleEmbedder::new(cfg)?))
        }

        "jina" => {
            let model = entry.model.as_deref().unwrap_or("jina-embeddings-v3");
            let cfg = match model {
                "jina-embeddings-v3" => EmbeddingRegistry::jina_embeddings_v3(),
                "jina-embeddings-v4" => EmbeddingRegistry::jina_embeddings_v4(),
                _ => EmbeddingRegistry::custom_openai_compatible(
                    "https://api.jina.ai/v1",
                    model,
                    entry.api_key_env.as_deref().unwrap_or("JINA_API_KEY"),
                    entry.dimensions,
                ),
            };
            Ok(Arc::new(OpenAICompatibleEmbedder::new(cfg)?))
        }

        "mistral" => {
            let cfg = EmbeddingRegistry::mistral_embed();
            Ok(Arc::new(OpenAICompatibleEmbedder::new(cfg)?))
        }

        "ollama" => {
            let model = entry.model.as_deref().unwrap_or("nomic-embed-text");
            let cfg = EmbeddingRegistry::ollama(model);
            Ok(Arc::new(OpenAICompatibleEmbedder::new(cfg)?))
        }

        // OpenAI-compatible (explicit or fallback for unknown types)
        "openai-compat" | _ => {
            let base_url = entry.base_url.as_deref().unwrap_or("http://localhost:11434/v1");
            let model = entry.model.as_deref().unwrap_or("unknown");

            let api_key = if let Some(ref key) = entry.api_key {
                ApiKeySource::Value(key.clone())
            } else if let Some(ref env_var) = entry.api_key_env {
                ApiKeySource::Env(env_var.clone())
            } else {
                ApiKeySource::None
            };

            if entry.provider_type != "openai-compat" {
                tracing::warn!(
                    alias = alias,
                    provider_type = entry.provider_type.as_str(),
                    "Unknown provider type, trying openai-compat"
                );
            }

            let cfg = OpenAICompatibleConfig {
                base_url: base_url.into(),
                model: model.into(),
                api_key,
                dimensions: Some(entry.dimensions),
                encoding_format: None,
                timeout_secs: entry.timeout_secs,
                max_batch_size: entry.max_batch_size,
                extra_headers: entry.extra_headers.clone(),
                model_info: EmbeddingModelInfo {
                    provider: entry.provider_type.clone(),
                    model_name: model.into(),
                    dimensions: entry.dimensions,
                    max_input_tokens: entry.max_input_tokens,
                    cost_per_million_tokens: entry.cost_per_million_tokens,
                    supports_shortening: entry.supports_mrl,
                    normalized_output: true,
                },
            };

            Ok(Arc::new(OpenAICompatibleEmbedder::new(cfg)?))
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
[embeddings]
active = "local-hash"

[embeddings.providers.local-hash]
type = "hash"
dimensions = 256
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        assert_eq!(config.embeddings.active, "local-hash");
        assert_eq!(config.embeddings.providers.len(), 1);
        let entry = &config.embeddings.providers["local-hash"];
        assert_eq!(entry.provider_type, "hash");
        assert_eq!(entry.dimensions, 256);
    }

    #[test]
    fn test_parse_multi_provider_config() {
        let toml = r#"
[embeddings]
active = "gemini-2"

[embeddings.providers.openai-large]
type = "openai-compat"
base_url = "https://api.openai.com/v1"
api_key_env = "OPENAI_API_KEY"
model = "text-embedding-3-large"
dimensions = 3072
max_input_tokens = 8191
cost_per_million_tokens = 0.13

[embeddings.providers.gemini-2]
type = "gemini"
api_key_env = "GEMINI_API_KEY"
model = "gemini-embedding-2-preview"
dimensions = 3072

[embeddings.providers.ollama-nomic]
type = "ollama"
model = "nomic-embed-text"
dimensions = 768
is_local = true
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        assert_eq!(config.embeddings.providers.len(), 3);
        assert_eq!(config.embeddings.active, "gemini-2");

        let openai = &config.embeddings.providers["openai-large"];
        assert_eq!(openai.provider_type, "openai-compat");
        assert_eq!(openai.dimensions, 3072);

        let ollama = &config.embeddings.providers["ollama-nomic"];
        assert!(ollama.is_local);
    }

    #[test]
    fn test_build_hash_from_config() {
        let toml = r#"
[embeddings]
active = "test-hash"

[embeddings.providers.test-hash]
type = "hash"
dimensions = 128
"#;
        let embedder = build_from_toml_str(toml).unwrap();
        assert_eq!(embedder.dimensions(), 128);
    }

    #[test]
    fn test_unknown_type_fallback() {
        let toml = r#"
[embeddings]
active = "future-ai"

[embeddings.providers.future-ai]
type = "some-future-provider"
base_url = "http://localhost:9999/v1"
model = "future-embed-v1"
dimensions = 4096
"#;
        // Should succeed — unknown type falls back to openai-compat
        let embedder = build_from_toml_str(toml).unwrap();
        assert_eq!(embedder.dimensions(), 4096);
    }

    #[test]
    fn test_missing_active_provider() {
        let toml = r#"
[embeddings]
active = "nonexistent"

[embeddings.providers.my-model]
type = "hash"
dimensions = 256
"#;
        let result = build_from_toml_str(toml);
        assert!(result.is_err());
        let err_msg = match result {
            Err(e) => e.to_string(),
            Ok(_) => panic!("Expected error for nonexistent provider"),
        };
        assert!(err_msg.contains("nonexistent"));
    }

    #[test]
    fn test_parse_with_extra_headers() {
        let toml = r#"
[embeddings]
active = "azure"

[embeddings.providers.azure]
type = "openai-compat"
base_url = "https://myresource.openai.azure.com/openai/deployments/my-embed"
api_key_env = "AZURE_OPENAI_API_KEY"
model = "text-embedding-3-small"
dimensions = 1536

[embeddings.providers.azure.extra_headers]
"api-version" = "2024-02-01"
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let azure = &config.embeddings.providers["azure"];
        assert_eq!(azure.extra_headers.get("api-version"), Some(&"2024-02-01".to_string()));
    }
}
