// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! OpenAI embeddings client.

use async_trait::async_trait;

use super::traits::{Embedder, EmbedderError, EmbedderResult};

/// OpenAI embeddings client using the `async-openai` crate.
pub struct OpenAiEmbedder {
    client: async_openai::Client<async_openai::config::OpenAIConfig>,
    model: String,
    dimensions: usize,
}

impl OpenAiEmbedder {
    /// Create a new OpenAI embedder with the given model.
    pub fn new(model: impl Into<String>) -> EmbedderResult<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| EmbedderError::ConfigError("OPENAI_API_KEY not set".into()))?;

        let config = async_openai::config::OpenAIConfig::new().with_api_key(api_key);
        let client = async_openai::Client::with_config(config);
        let model = model.into();

        let dimensions = match model.as_str() {
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            _ => 1536,
        };

        Ok(Self {
            client,
            model,
            dimensions,
        })
    }

    /// Create with the default text-embedding-3-small model.
    pub fn default_model() -> EmbedderResult<Self> {
        Self::new("text-embedding-3-small")
    }
}

impl std::fmt::Debug for OpenAiEmbedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiEmbedder")
            .field("model", &self.model)
            .field("dimensions", &self.dimensions)
            .finish()
    }
}

#[async_trait]
impl Embedder for OpenAiEmbedder {
    fn model_name(&self) -> &str {
        &self.model
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    async fn embed_batch(&self, texts: &[String]) -> EmbedderResult<Vec<Vec<f32>>> {
        use async_openai::types::CreateEmbeddingRequestArgs;

        let texts_vec: Vec<String> = texts.to_vec();
        let request = CreateEmbeddingRequestArgs::default()
            .model(&self.model)
            .input(&texts_vec)
            .build()
            .map_err(|e| EmbedderError::ApiError(e.to_string()))?;

        let response = self
            .client
            .embeddings()
            .create(request)
            .await
            .map_err(|e| EmbedderError::ApiError(e.to_string()))?;

        let embeddings: Vec<Vec<f32>> = response.data.into_iter().map(|d| d.embedding).collect();

        Ok(embeddings)
    }
}
