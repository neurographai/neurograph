// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Google Gemini LLM client.
//!
//! Key differences from OpenAI:
//! - URL: `…/models/{model}:generateContent?key={apiKey}`
//! - Role mapping: "assistant" → "model"
//! - Messages are "contents" with "parts"
//! - System instruction is a separate top-level field
//! - Supports: gemini-2.5-pro (1M ctx), gemini-2.5-flash, gemini-2.0-flash

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::llm::config::LlmConfig;
use crate::llm::traits::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, LlmProvider, LlmResult, LlmUsage,
    ModelCapabilities,
};

/// Google Gemini client using the generateContent API.
pub struct GeminiClient {
    client: reqwest::Client,
    config: LlmConfig,
    api_key: String,
}

impl GeminiClient {
    pub fn new(config: LlmConfig) -> LlmResult<Self> {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("GEMINI_API_KEY").ok())
            .ok_or_else(|| LlmError::ConfigError("GEMINI_API_KEY not set".into()))?;

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
        let mut config = if model.contains("pro") {
            LlmConfig::gemini_pro()
        } else {
            LlmConfig::gemini_flash()
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

    fn endpoint(&self) -> String {
        let base = self
            .config
            .base_url
            .as_deref()
            .unwrap_or("https://generativelanguage.googleapis.com/v1beta");
        format!(
            "{}/models/{}:generateContent?key={}",
            base, self.config.model, self.api_key
        )
    }
}

impl std::fmt::Debug for GeminiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeminiClient")
            .field("model", &self.config.model)
            .finish()
    }
}

// ── Request types for Gemini API ────────────────────────────────────

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "systemInstruction")]
    system_instruction: Option<GeminiSystemInstruction>,
    #[serde(rename = "generationConfig")]
    generation_config: GeminiGenerationConfig,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiSystemInstruction {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    temperature: f32,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "responseMimeType"
    )]
    response_mime_type: Option<String>,
}

// ── Response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsageMetadata>,
    error: Option<GeminiApiError>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiResponseContent>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponseContent {
    parts: Option<Vec<GeminiPart>>,
}

#[derive(Debug, Deserialize)]
struct GeminiUsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: Option<u32>,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GeminiApiError {
    message: String,
}

// ── LlmClient Implementation ───────────────────────────────────────

#[async_trait]
impl LlmClient for GeminiClient {
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
        LlmProvider::Gemini
    }

    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            context_window: 1_000_000,
            max_output_tokens: 65_536,
            supports_streaming: true,
            supports_function_calling: true,
            supports_structured_output: true,
            supports_vision: true,
            supports_reasoning: self.config.model.contains("pro"),
        }
    }

    async fn complete(&self, request: CompletionRequest) -> LlmResult<CompletionResponse> {
        let start = Instant::now();

        // Gemini: system instruction is a separate top-level field
        let system_instruction = match (&request.system_prompt, request.json_mode) {
            (Some(sys), true) => Some(GeminiSystemInstruction {
                parts: vec![GeminiPart {
                    text: format!(
                        "{}\n\nIMPORTANT: Respond with valid JSON only.",
                        sys
                    ),
                }],
            }),
            (Some(sys), false) => Some(GeminiSystemInstruction {
                parts: vec![GeminiPart {
                    text: sys.clone(),
                }],
            }),
            (None, true) => Some(GeminiSystemInstruction {
                parts: vec![GeminiPart {
                    text: "Respond with valid JSON only.".to_string(),
                }],
            }),
            (None, false) => None,
        };

        // Gemini uses "user" and "model" roles (not "assistant")
        let contents = vec![GeminiContent {
            role: "user".to_string(),
            parts: vec![GeminiPart {
                text: request.user_prompt.clone(),
            }],
        }];

        let body = GeminiRequest {
            contents,
            system_instruction,
            generation_config: GeminiGenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request.max_tokens.unwrap_or(self.config.max_tokens),
                response_mime_type: if request.json_mode {
                    Some("application/json".to_string())
                } else {
                    None
                },
            },
        };

        let response = self
            .client
            .post(&self.endpoint())
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
            return Err(LlmError::ApiError(format!(
                "Gemini API error ({}): {}",
                status,
                &response_body[..response_body.len().min(500)]
            )));
        }

        let parsed: GeminiResponse = serde_json::from_str(&response_body)
            .map_err(|e| LlmError::ApiError(format!("Failed to parse response: {}", e)))?;

        if let Some(err) = parsed.error {
            return Err(LlmError::ApiError(format!("Gemini error: {}", err.message)));
        }

        let latency_ms = start.elapsed().as_millis() as u64;

        let content = parsed
            .candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.content)
            .and_then(|c| c.parts)
            .and_then(|p| p.into_iter().next())
            .map(|p| p.text)
            .unwrap_or_default();

        let input_tokens = parsed
            .usage_metadata
            .as_ref()
            .and_then(|u| u.prompt_token_count)
            .unwrap_or(0);
        let output_tokens = parsed
            .usage_metadata
            .as_ref()
            .and_then(|u| u.candidates_token_count)
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
