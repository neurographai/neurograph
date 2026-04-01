// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Anthropic Claude LLM client.
//!
//! Key differences from OpenAI:
//! - Header: `x-api-key` (not Bearer)
//! - Header: `anthropic-version: 2023-06-01`
//! - System prompt is a top-level field, NOT in messages array
//! - `max_tokens` is REQUIRED in request body
//! - Supports: claude-opus-4-5, claude-sonnet-4-5, claude-haiku-3-5

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::llm::config::LlmConfig;
use crate::llm::traits::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, LlmProvider, LlmResult, LlmUsage,
    ModelCapabilities,
};

/// Anthropic Claude client using the Messages API.
pub struct AnthropicClient {
    client: reqwest::Client,
    config: LlmConfig,
    api_key: String,
}

impl AnthropicClient {
    /// Create a new Anthropic client from config.
    pub fn new(config: LlmConfig) -> LlmResult<Self> {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .ok_or_else(|| {
                LlmError::ConfigError("ANTHROPIC_API_KEY not set".into())
            })?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| LlmError::ApiError(format!("HTTP client error: {}", e)))?;

        Ok(Self {
            client,
            config,
            api_key,
        })
    }

    /// Create with an explicit API key (for settings test flow).
    pub fn with_key(api_key: String, model: &str) -> LlmResult<Self> {
        let mut config = if model.contains("haiku") {
            LlmConfig::anthropic_haiku()
        } else {
            LlmConfig::anthropic_sonnet()
        };
        config.model = model.to_string();
        config.api_key = Some(api_key.clone());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| LlmError::ApiError(format!("HTTP client error: {}", e)))?;

        Ok(Self {
            client,
            config,
            api_key,
        })
    }
}

impl std::fmt::Debug for AnthropicClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnthropicClient")
            .field("model", &self.config.model)
            .finish()
    }
}

// ── Request types for Anthropic Messages API ────────────────────────

#[derive(Debug, Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<ContentBlock>,
}

#[derive(Debug, Serialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

// ── Response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ResponseContent>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct AnthropicErrorResponse {
    error: AnthropicErrorDetail,
}

#[derive(Debug, Deserialize)]
struct AnthropicErrorDetail {
    message: String,
}

// ── LlmClient Implementation ───────────────────────────────────────

#[async_trait]
impl LlmClient for AnthropicClient {
    fn model_name(&self) -> &str {
        &self.config.model
    }

    fn cost_per_token(&self) -> (f64, f64) {
        (
            self.config.input_cost_per_million / 1_000_000.0,
            self.config.output_cost_per_million / 1_000_000.0,
        )
    }

    fn provider(&self) -> LlmProvider {
        LlmProvider::Anthropic
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            context_window: 200_000,
            max_output_tokens: if self.config.model.contains("opus") {
                32_000
            } else {
                16_000
            },
            supports_streaming: true,
            supports_function_calling: true,
            supports_structured_output: true,
            supports_vision: true,
            supports_reasoning: false,
        }
    }

    async fn complete(&self, request: CompletionRequest) -> LlmResult<CompletionResponse> {
        let start = Instant::now();

        // Anthropic: system prompt is a top-level field, not in messages
        let system = match (&request.system_prompt, request.json_mode) {
            (Some(sys), true) => Some(format!(
                "{}\n\nIMPORTANT: You MUST respond with valid JSON only. No markdown, no explanation.",
                sys
            )),
            (Some(sys), false) => Some(sys.clone()),
            (None, true) => Some(
                "You MUST respond with valid JSON only. No markdown, no explanation.".to_string(),
            ),
            (None, false) => None,
        };

        let messages = vec![AnthropicMessage {
            role: "user".to_string(),
            content: vec![ContentBlock {
                content_type: "text".to_string(),
                text: request.user_prompt.clone(),
            }],
        }];

        let body = AnthropicRequest {
            model: &self.config.model,
            max_tokens: request.max_tokens.unwrap_or(self.config.max_tokens),
            system,
            messages,
            temperature: Some(request.temperature),
        };

        let base_url = self
            .config
            .base_url
            .as_deref()
            .unwrap_or("https://api.anthropic.com/v1");

        let response = self
            .client
            .post(format!("{}/messages", base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::ApiError(format!("Request failed: {}", e)))?;

        let status = response.status();
        let response_body = response
            .text()
            .await
            .map_err(|e| LlmError::ApiError(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            if status.as_u16() == 429 {
                return Err(LlmError::RateLimited {
                    retry_after_ms: 1000,
                });
            }
            let error_msg =
                if let Ok(err) = serde_json::from_str::<AnthropicErrorResponse>(&response_body) {
                    err.error.message
                } else {
                    response_body
                };
            return Err(LlmError::ApiError(format!(
                "Anthropic API error ({}): {}",
                status, error_msg
            )));
        }

        let parsed: AnthropicResponse = serde_json::from_str(&response_body)
            .map_err(|e| LlmError::ApiError(format!("Failed to parse response: {}", e)))?;

        let latency_ms = start.elapsed().as_millis() as u64;

        let content = parsed
            .content
            .first()
            .and_then(|c| c.text.clone())
            .unwrap_or_default();

        let cost_usd = self
            .config
            .calculate_cost(parsed.usage.input_tokens, parsed.usage.output_tokens);

        Ok(CompletionResponse {
            content,
            usage: LlmUsage {
                input_tokens: parsed.usage.input_tokens,
                output_tokens: parsed.usage.output_tokens,
                cost_usd,
                latency_ms,
                model: self.config.model.clone(),
            },
        })
    }
}
