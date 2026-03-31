// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Universal OpenAI-compatible embedding client.
//!
//! This single client handles OpenAI, Gemini, Cohere, Voyage, Jina,
//! Mistral, Azure, Ollama (/v1), vLLM, LM Studio, and any future
//! provider that follows the OpenAI `/v1/embeddings` format.
//!
//! # Usage
//!
//! ```rust,no_run
//! use neurograph_core::embedders::openai_compatible::*;
//! use neurograph_core::embedders::providers::EmbeddingRegistry;
//!
//! let config = EmbeddingRegistry::openai_text_embedding_3_small();
//! let embedder = OpenAICompatibleEmbedder::new(config).unwrap();
//! ```

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use super::traits::{Embedder, EmbedderError, EmbedderResult, EmbeddingModelInfo};

// ── Configuration ───────────────────────────────────────────────────────

/// Configuration for ANY OpenAI-compatible embedding API.
///
/// # Supported Providers
///
/// | Provider | `base_url` |
/// |----------|-----------|
/// | OpenAI | `https://api.openai.com/v1` |
/// | Gemini | `https://generativelanguage.googleapis.com/v1beta/openai` |
/// | Cohere | `https://api.cohere.com/compatibility/v1` |
/// | Voyage | `https://api.voyageai.com/v1` |
/// | Jina | `https://api.jina.ai/v1` |
/// | Mistral | `https://api.mistral.ai/v1` |
/// | Azure | `https://{resource}.openai.azure.com/openai/deployments/{model}` |
/// | Ollama | `http://localhost:11434/v1` |
/// | vLLM | `http://localhost:8000/v1` |
/// | LM Studio | `http://localhost:1234/v1` |
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAICompatibleConfig {
    /// API base URL (NO trailing slash, NO `/v1/embeddings`).
    pub base_url: String,

    /// Model name to pass in the API request.
    pub model: String,

    /// API key source.
    pub api_key: ApiKeySource,

    /// Optional: override output dimensions (for Matryoshka models).
    pub dimensions: Option<usize>,

    /// Optional: encoding format ("float" or "base64").
    pub encoding_format: Option<String>,

    /// Request timeout in seconds.
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,

    /// Max texts per batch request.
    #[serde(default = "default_max_batch")]
    pub max_batch_size: usize,

    /// Custom headers (e.g., Azure `api-version`, Anthropic beta headers).
    #[serde(default)]
    pub extra_headers: HashMap<String, String>,

    /// Model metadata.
    pub model_info: EmbeddingModelInfo,
}

fn default_timeout_secs() -> u64 {
    30
}
fn default_max_batch() -> usize {
    100
}

impl Default for OpenAICompatibleConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434/v1".into(),
            model: "nomic-embed-text".into(),
            api_key: ApiKeySource::None,
            dimensions: None,
            encoding_format: None,
            timeout_secs: 30,
            max_batch_size: 100,
            extra_headers: HashMap::new(),
            model_info: EmbeddingModelInfo {
                provider: "local".into(),
                model_name: "nomic-embed-text".into(),
                dimensions: 768,
                max_input_tokens: 8192,
                cost_per_million_tokens: 0.0,
                supports_shortening: false,
                normalized_output: false,
            },
        }
    }
}

/// How to resolve the API key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiKeySource {
    /// Direct value (NOT recommended for production).
    Value(String),
    /// Read from environment variable.
    Env(String),
    /// No API key (local models).
    None,
}

impl ApiKeySource {
    /// Resolve to an actual key string, if available.
    pub fn resolve(&self) -> Option<String> {
        match self {
            Self::Value(v) => Some(v.clone()),
            Self::Env(var) => std::env::var(var).ok(),
            Self::None => None,
        }
    }
}

// ── Client Implementation ───────────────────────────────────────────────

/// Universal client for any OpenAI-compatible embedding API.
pub struct OpenAICompatibleEmbedder {
    config: OpenAICompatibleConfig,
    client: Client,
}

impl std::fmt::Debug for OpenAICompatibleEmbedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAICompatibleEmbedder")
            .field("provider", &self.config.model_info.provider)
            .field("model", &self.config.model)
            .field("base_url", &self.config.base_url)
            .field("dimensions", &self.config.model_info.dimensions)
            .finish()
    }
}

// ── OpenAI API types ────────────────────────────────────────────────────

#[derive(Serialize)]
struct EmbeddingRequest<'a> {
    model: &'a str,
    input: EmbeddingInput<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding_format: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum EmbeddingInput<'a> {
    Single(&'a str),
    Batch(&'a [String]),
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
    #[allow(dead_code)]
    model: Option<String>,
    #[allow(dead_code)]
    usage: Option<EmbeddingUsage>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Deserialize)]
struct EmbeddingUsage {
    #[allow(dead_code)]
    prompt_tokens: Option<u64>,
    #[allow(dead_code)]
    total_tokens: Option<u64>,
}

#[derive(Deserialize)]
struct ApiErrorResponse {
    error: ApiErrorDetail,
}

#[derive(Deserialize)]
struct ApiErrorDetail {
    message: String,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    error_type: Option<String>,
}

// ── Implementation ──────────────────────────────────────────────────────

impl OpenAICompatibleEmbedder {
    /// Create a new embedder from configuration.
    pub fn new(config: OpenAICompatibleConfig) -> Result<Self, EmbedderError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| EmbedderError::ConfigError(e.to_string()))?;

        Ok(Self { config, client })
    }

    /// Build the full embeddings endpoint URL.
    fn embeddings_url(&self) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        format!("{}/embeddings", base)
    }

    /// Make the actual API call.
    async fn call_api(
        &self,
        input: EmbeddingInput<'_>,
    ) -> Result<EmbeddingResponse, EmbedderError> {
        let request_body = EmbeddingRequest {
            model: &self.config.model,
            input,
            dimensions: self.config.dimensions,
            encoding_format: self.config.encoding_format.as_deref(),
        };

        let mut req = self.client.post(&self.embeddings_url()).json(&request_body);

        // Add auth header
        if let Some(key) = self.config.api_key.resolve() {
            req = req.bearer_auth(&key);
        }

        // Add any extra headers (Azure api-version, etc.)
        for (k, v) in &self.config.extra_headers {
            req = req.header(k, v);
        }

        let response = req
            .send()
            .await
            .map_err(|e| EmbedderError::ApiError(e.to_string()))?;

        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(1000);
            return Err(EmbedderError::RateLimited {
                retry_after_ms: retry_after,
            });
        }

        if status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::FORBIDDEN
        {
            return Err(EmbedderError::AuthError);
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            // Try to parse structured error
            if let Ok(api_err) = serde_json::from_str::<ApiErrorResponse>(&error_text) {
                return Err(EmbedderError::ApiError(api_err.error.message));
            }
            return Err(EmbedderError::ApiError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        response
            .json::<EmbeddingResponse>()
            .await
            .map_err(|e| EmbedderError::ApiError(format!("Failed to parse response: {}", e)))
    }
}

#[async_trait]
impl Embedder for OpenAICompatibleEmbedder {
    fn model_name(&self) -> &str {
        &self.config.model_info.model_name
    }

    fn dimensions(&self) -> usize {
        self.config
            .dimensions
            .unwrap_or(self.config.model_info.dimensions)
    }

    fn model_info(&self) -> EmbeddingModelInfo {
        let mut info = self.config.model_info.clone();
        if let Some(dims) = self.config.dimensions {
            info.dimensions = dims;
        }
        info
    }

    async fn embed_one(&self, text: &str) -> EmbedderResult<Vec<f32>> {
        let response = self.call_api(EmbeddingInput::Single(text)).await?;

        response
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| EmbedderError::ApiError("Empty response".into()))
    }

    async fn embed_batch(&self, texts: &[String]) -> EmbedderResult<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let mut all_results = Vec::with_capacity(texts.len());

        // Chunk into max_batch_size groups
        for chunk in texts.chunks(self.config.max_batch_size) {
            let response = self.call_api(EmbeddingInput::Batch(chunk)).await?;

            // Sort by index to preserve order
            let mut sorted = response.data;
            sorted.sort_by_key(|d| d.index);

            all_results.extend(sorted.into_iter().map(|d| d.embedding));
        }

        Ok(all_results)
    }

    async fn health_check(&self) -> EmbedderResult<()> {
        self.embed_one("health check").await.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = OpenAICompatibleConfig::default();
        assert_eq!(config.base_url, "http://localhost:11434/v1");
        assert_eq!(config.model, "nomic-embed-text");
        assert!(matches!(config.api_key, ApiKeySource::None));
    }

    #[test]
    fn test_embeddings_url() {
        let config = OpenAICompatibleConfig {
            base_url: "https://api.openai.com/v1".into(),
            ..Default::default()
        };
        let embedder = OpenAICompatibleEmbedder::new(config).unwrap();
        assert_eq!(
            embedder.embeddings_url(),
            "https://api.openai.com/v1/embeddings"
        );
    }

    #[test]
    fn test_embeddings_url_trailing_slash() {
        let config = OpenAICompatibleConfig {
            base_url: "https://api.openai.com/v1/".into(),
            ..Default::default()
        };
        let embedder = OpenAICompatibleEmbedder::new(config).unwrap();
        assert_eq!(
            embedder.embeddings_url(),
            "https://api.openai.com/v1/embeddings"
        );
    }

    #[test]
    fn test_api_key_resolve_none() {
        assert!(ApiKeySource::None.resolve().is_none());
    }

    #[test]
    fn test_api_key_resolve_value() {
        let key = ApiKeySource::Value("sk-test".into());
        assert_eq!(key.resolve(), Some("sk-test".into()));
    }

    #[test]
    fn test_api_key_resolve_env_missing() {
        let key = ApiKeySource::Env("NEUROGRAPH_TEST_NONEXISTENT_KEY_12345".into());
        assert!(key.resolve().is_none());
    }

    #[test]
    fn test_debug_format() {
        let config = OpenAICompatibleConfig::default();
        let embedder = OpenAICompatibleEmbedder::new(config).unwrap();
        let debug = format!("{:?}", embedder);
        assert!(debug.contains("OpenAICompatibleEmbedder"));
        assert!(debug.contains("nomic-embed-text"));
    }
}
