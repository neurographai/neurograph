// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! OpenAI LLM client implementation using raw `reqwest`.
//!
//! Replaces the former `async-openai` dependency with direct HTTP calls
//! to the OpenAI Chat Completions API, consistent with how
//! `openai_compatible.rs` handles embeddings.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use super::config::LlmConfig;
use super::traits::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, LlmResult, LlmUsage,
};

/// OpenAI client using raw `reqwest` for Chat Completions.
pub struct OpenAiClient {
    client: reqwest::Client,
    config: LlmConfig,
    api_key: String,
    base_url: String,
}

impl OpenAiClient {
    /// Create a new OpenAI client with the given config.
    pub fn new(config: LlmConfig) -> LlmResult<Self> {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| {
                LlmError::ConfigError(
                    "OPENAI_API_KEY not set. Set it via env var or config.".into(),
                )
            })?;

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| LlmError::ApiError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            config,
            api_key,
            base_url,
        })
    }

    /// Create with the default GPT-4o-mini config.
    pub fn default_model() -> LlmResult<Self> {
        Self::new(LlmConfig::gpt4o_mini())
    }
}

impl std::fmt::Debug for OpenAiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiClient")
            .field("model", &self.config.model)
            .field("base_url", &self.base_url)
            .finish()
    }
}

// ── Request/Response types for OpenAI Chat Completions ──────────────────

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
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

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChatChoiceMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    error: ApiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    message: String,
}

// ── Trait Implementation ────────────────────────────────────────────────

#[async_trait]
impl LlmClient for OpenAiClient {
    fn model_name(&self) -> &str {
        &self.config.model
    }

    fn cost_per_token(&self) -> (f64, f64) {
        (
            self.config.input_cost_per_million / 1_000_000.0,
            self.config.output_cost_per_million / 1_000_000.0,
        )
    }

    fn provider(&self) -> super::traits::LlmProvider {
        super::traits::LlmProvider::OpenAI
    }

    async fn complete(&self, request: CompletionRequest) -> LlmResult<CompletionResponse> {
        let start = Instant::now();

        let mut messages: Vec<ChatMessage> = Vec::new();

        // Build system prompt, adding JSON mode hint if needed
        let system_content = match (&request.system_prompt, request.json_mode) {
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

        if let Some(system) = system_content {
            messages.push(ChatMessage {
                role: "system".into(),
                content: system,
            });
        }

        messages.push(ChatMessage {
            role: "user".into(),
            content: request.user_prompt.clone(),
        });

        let chat_request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            temperature: Some(request.temperature),
            max_tokens: request.max_tokens.map(|t| t as u32),
            response_format: if request.json_mode {
                Some(ResponseFormat {
                    format_type: "json_object".into(),
                })
            } else {
                None
            },
        };

        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&chat_request)
            .send()
            .await
            .map_err(|e| LlmError::ApiError(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| LlmError::ApiError(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            let error_msg = if let Ok(err) = serde_json::from_str::<ApiErrorResponse>(&body) {
                err.error.message
            } else {
                body
            };
            return Err(LlmError::ApiError(format!(
                "API error ({}): {}",
                status, error_msg
            )));
        }

        let chat_response: ChatResponse = serde_json::from_str(&body)
            .map_err(|e| LlmError::ApiError(format!("Failed to parse response: {}", e)))?;

        let latency_ms = start.elapsed().as_millis() as u64;

        let content = chat_response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        let input_tokens = chat_response
            .usage
            .as_ref()
            .map(|u| u.prompt_tokens)
            .unwrap_or(0);
        let output_tokens = chat_response
            .usage
            .as_ref()
            .map(|u| u.completion_tokens)
            .unwrap_or(0);

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
