// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Generic OpenAI-compatible LLM client.
//!
//! Works with any API that implements the OpenAI chat completions format:
//! - Ollama (`http://localhost:11434/v1`)
//! - vLLM, LiteLLM, LocalAI, etc.
//! - Azure OpenAI

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use super::config::LlmConfig;
use super::traits::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, LlmResult, LlmUsage,
};

/// Request body for OpenAI-compatible APIs.
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

/// Response body from OpenAI-compatible APIs.
#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Debug, Deserialize)]
struct ChatMessageResponse {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

/// Generic client for any OpenAI-compatible API.
///
/// Uses raw `reqwest` HTTP calls instead of the `async-openai` crate,
/// providing maximum compatibility with non-standard endpoints.
pub struct GenericLlmClient {
    http: reqwest::Client,
    config: LlmConfig,
}

impl GenericLlmClient {
    /// Create a new generic LLM client.
    pub fn new(config: LlmConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        Self { http, config }
    }

    /// Create a client for Ollama with the given model.
    pub fn ollama(model: impl Into<String>) -> Self {
        Self::new(LlmConfig::ollama(model))
    }

    fn base_url(&self) -> &str {
        self.config
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1")
    }
}

impl std::fmt::Debug for GenericLlmClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenericLlmClient")
            .field("model", &self.config.model)
            .field("base_url", &self.base_url())
            .finish()
    }
}

#[async_trait]
impl LlmClient for GenericLlmClient {
    fn model_name(&self) -> &str {
        &self.config.model
    }

    fn cost_per_token(&self) -> (f64, f64) {
        (
            self.config.input_cost_per_million / 1_000_000.0,
            self.config.output_cost_per_million / 1_000_000.0,
        )
    }

    async fn complete(&self, request: CompletionRequest) -> LlmResult<CompletionResponse> {
        let start = Instant::now();

        let mut messages = Vec::new();

        if let Some(ref system) = request.system_prompt {
            messages.push(ChatMessage {
                role: "system".into(),
                content: system.clone(),
            });
        }

        messages.push(ChatMessage {
            role: "user".into(),
            content: request.user_prompt.clone(),
        });

        let body = ChatRequest {
            model: self.config.model.clone(),
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            response_format: if request.json_mode {
                Some(ResponseFormat {
                    format_type: "json_object".into(),
                })
            } else {
                None
            },
        };

        let url = format!("{}/chat/completions", self.base_url());

        let mut req = self.http.post(&url).json(&body);

        if let Some(ref api_key) = self.config.api_key {
            req = req.bearer_auth(api_key);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| LlmError::ApiError(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if status.as_u16() == 429 {
                return Err(LlmError::RateLimited {
                    retry_after_ms: 1000,
                });
            }
            return Err(LlmError::ApiError(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let chat_response: ChatResponse = resp
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        let latency_ms = start.elapsed().as_millis() as u64;

        let content = chat_response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        let (input_tokens, output_tokens) = chat_response
            .usage
            .map(|u| (u.prompt_tokens, u.completion_tokens))
            .unwrap_or((0, 0));

        let cost_usd = self.config.calculate_cost(input_tokens, output_tokens);

        Ok(CompletionResponse {
            content,
            usage: LlmUsage {
                input_tokens,
                output_tokens,
                cost_usd,
                latency_ms,
                model: self.config.model.clone(),
            },
        })
    }
}
