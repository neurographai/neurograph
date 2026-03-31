// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Ollama embedding provider — integrates with the existing Embedder trait.
//!
//! Connects to a local Ollama instance for embedding generation.
//! Default endpoint: http://localhost:11434/api/embeddings

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::traits::{Embedder, EmbedderError, EmbedderResult};

/// Ollama-based embedding provider.
pub struct OllamaEmbedder {
    client: reqwest::Client,
    base_url: String,
    model: String,
    dimension: std::sync::atomic::AtomicUsize,
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embedding: Vec<f64>,
}

impl OllamaEmbedder {
    /// Create a new Ollama embedder for the given model.
    ///
    /// Reads `OLLAMA_HOST` env var for the base URL.
    /// Defaults to `http://localhost:11434`.
    pub fn new(model: &str) -> Result<Self, EmbedderError> {
        let base_url = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());

        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| EmbedderError::ConfigError(e.to_string()))?,
            base_url,
            model: model.to_string(),
            dimension: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    /// Create with explicit base URL.
    pub fn with_url(model: &str, base_url: &str) -> Result<Self, EmbedderError> {
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| EmbedderError::ConfigError(e.to_string()))?,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            dimension: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    /// Test connectivity to the Ollama server.
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        self.client.get(&url).send().await.map(|r| r.status().is_success()).unwrap_or(false)
    }

    /// Embed a single text (standalone, not through the trait).
    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>, EmbedderError> {
        let url = format!("{}/api/embeddings", self.base_url);
        let request = EmbedRequest {
            model: &self.model,
            prompt: text,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                EmbedderError::ApiError(format!(
                    "Failed to connect to Ollama at {}: {}. Is Ollama running?",
                    self.base_url, e
                ))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(EmbedderError::ApiError(format!(
                "Ollama returned {}: {}. Is '{}' pulled? Run: ollama pull {}",
                status, body, self.model, self.model
            )));
        }

        let embed_response: EmbedResponse = response.json().await.map_err(|e| {
            EmbedderError::ApiError(format!("Failed to parse Ollama response: {}", e))
        })?;

        let embedding: Vec<f32> = embed_response.embedding.iter().map(|&v| v as f32).collect();
        self.dimension
            .store(embedding.len(), std::sync::atomic::Ordering::Relaxed);

        Ok(embedding)
    }
}

#[async_trait]
impl Embedder for OllamaEmbedder {
    fn model_name(&self) -> &str {
        &self.model
    }

    fn dimensions(&self) -> usize {
        let dim = self.dimension.load(std::sync::atomic::Ordering::Relaxed);
        if dim > 0 { dim } else { 768 } // default for nomic-embed-text
    }

    async fn embed_batch(&self, texts: &[String]) -> EmbedderResult<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed_text(text).await?);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_ollama_embedder() {
        let embedder = OllamaEmbedder::new("nomic-embed-text").unwrap();
        assert_eq!(embedder.model_name(), "nomic-embed-text");
        assert_eq!(embedder.dimensions(), 768);
    }

    #[test]
    fn test_custom_url() {
        let embedder = OllamaEmbedder::with_url("mxbai-embed-large", "http://192.168.1.100:11434").unwrap();
        assert_eq!(embedder.base_url, "http://192.168.1.100:11434");
    }
}
