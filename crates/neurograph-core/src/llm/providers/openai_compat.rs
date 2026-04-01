// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! OpenAI-compatible LLM client for xAI Grok and Groq.
//!
//! Both xAI and Groq implement the exact OpenAI `/chat/completions` format,
//! just with different base URLs and API keys. This client wraps the existing
//! `GenericLlmClient` with proper provider identity and explicit constructor
//! helpers for each service.

use crate::llm::config::LlmConfig;
use crate::llm::generic::GenericLlmClient;
use crate::llm::traits::{
    CompletionRequest, CompletionResponse, LlmClient, LlmProvider, LlmResult, ModelCapabilities,
};
use async_trait::async_trait;

/// An OpenAI-compatible client with explicit provider identity.
///
/// Used for xAI Grok and Groq — both are API-compatible with OpenAI
/// but need different base URLs, keys, and provider tagging.
pub struct OpenAiCompatClient {
    inner: GenericLlmClient,
    provider_id: LlmProvider,
}

impl OpenAiCompatClient {
    /// Create a Groq client (~500 t/s inference).
    pub fn groq(config: LlmConfig) -> Self {
        Self {
            inner: GenericLlmClient::new(config),
            provider_id: LlmProvider::Groq,
        }
    }

    /// Create a Groq client with explicit API key.
    pub fn groq_with_key(api_key: String, model: &str) -> Self {
        let mut config = LlmConfig::groq_llama();
        config.model = model.to_string();
        config.api_key = Some(api_key);
        Self {
            inner: GenericLlmClient::new(config),
            provider_id: LlmProvider::Groq,
        }
    }

    /// Create an xAI Grok client.
    pub fn xai(config: LlmConfig) -> Self {
        Self {
            inner: GenericLlmClient::new(config),
            provider_id: LlmProvider::XaiGrok,
        }
    }

    /// Create an xAI Grok client with explicit API key.
    pub fn xai_with_key(api_key: String, model: &str) -> Self {
        let mut config = LlmConfig::xai_grok_mini();
        config.model = model.to_string();
        config.api_key = Some(api_key);
        Self {
            inner: GenericLlmClient::new(config),
            provider_id: LlmProvider::XaiGrok,
        }
    }
}

impl std::fmt::Debug for OpenAiCompatClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiCompatClient")
            .field("provider", &self.provider_id)
            .field("model", &self.inner.model_name())
            .finish()
    }
}

#[async_trait]
impl LlmClient for OpenAiCompatClient {
    fn model_name(&self) -> &str {
        self.inner.model_name()
    }

    fn cost_per_token(&self) -> (f64, f64) {
        self.inner.cost_per_token()
    }

    fn provider(&self) -> LlmProvider {
        self.provider_id
    }

    fn capabilities(&self) -> ModelCapabilities {
        match self.provider_id {
            LlmProvider::Groq => ModelCapabilities {
                context_window: 128_000,
                max_output_tokens: 32_768,
                supports_streaming: true,
                supports_function_calling: true,
                supports_structured_output: true,
                supports_vision: false,
                supports_reasoning: self.inner.model_name().contains("deepseek")
                    || self.inner.model_name().contains("qwq"),
            },
            LlmProvider::XaiGrok => ModelCapabilities {
                context_window: 131_072,
                max_output_tokens: 131_072,
                supports_streaming: true,
                supports_function_calling: true,
                supports_structured_output: true,
                supports_vision: false,
                supports_reasoning: self.inner.model_name().contains("mini"),
            },
            _ => ModelCapabilities::default(),
        }
    }

    async fn complete(&self, request: CompletionRequest) -> LlmResult<CompletionResponse> {
        self.inner.complete(request).await
    }
}
