// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! LLM client trait definition.
//!
//! Influenced by Graphiti's `LLMClient` ABC (llm_client/client.py):
//! - `generate_response()` with JSON mode support
//! - Token tracking per prompt type
//!
//! Enhanced with:
//! - Structured output via generics (`complete_structured<T>`)
//! - Cost-per-token reporting for budget tracking
//! - Batch completion for parallelism
//! - Multi-provider routing (OpenAI, Anthropic, Gemini, xAI, Groq, Ollama)
//! - Health checks and capability detection

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Supported LLM providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    OpenAI,
    Anthropic,
    Gemini,
    XaiGrok,
    Groq,
    Ollama,
}

impl std::fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmProvider::OpenAI => write!(f, "openai"),
            LlmProvider::Anthropic => write!(f, "anthropic"),
            LlmProvider::Gemini => write!(f, "gemini"),
            LlmProvider::XaiGrok => write!(f, "xai"),
            LlmProvider::Groq => write!(f, "groq"),
            LlmProvider::Ollama => write!(f, "ollama"),
        }
    }
}

impl std::str::FromStr for LlmProvider {
    type Err = LlmError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(LlmProvider::OpenAI),
            "anthropic" => Ok(LlmProvider::Anthropic),
            "gemini" | "google" => Ok(LlmProvider::Gemini),
            "xai" | "grok" => Ok(LlmProvider::XaiGrok),
            "groq" => Ok(LlmProvider::Groq),
            "ollama" => Ok(LlmProvider::Ollama),
            _ => Err(LlmError::ConfigError(format!("Unknown provider: {}", s))),
        }
    }
}

/// Model capabilities for feature detection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub supports_streaming: bool,
    pub supports_function_calling: bool,
    pub supports_structured_output: bool,
    pub supports_vision: bool,
    pub supports_reasoning: bool,
}

/// Speed tier classification for latency-aware routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeedTier {
    /// < 100ms first token (Groq)
    Instant,
    /// < 500ms (GPT-4o-mini, Gemini Flash, Haiku)
    Fast,
    /// < 2s (GPT-4o, Claude Sonnet)
    Standard,
    /// > 2s (o3, Claude Opus, Gemini Pro)
    Slow,
}

/// Health status of a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub provider: LlmProvider,
    pub healthy: bool,
    pub latency_ms: u64,
    pub available_models: Vec<String>,
    pub error: Option<String>,
    pub checked_at: chrono::DateTime<chrono::Utc>,
}

/// A single token in a streaming response.
#[derive(Debug, Clone)]
pub struct StreamToken {
    pub token: String,
    pub is_done: bool,
}

/// Errors from LLM operations.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Rate limited: retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Model not available: {0}")]
    ModelNotAvailable(String),

    #[error("Budget exceeded: spent ${spent:.4}, limit ${limit:.4}")]
    BudgetExceeded { spent: f64, limit: f64 },

    #[error("All providers unavailable")]
    AllProvidersUnavailable,

    #[error("Unknown provider: {0}")]
    UnknownProvider(String),
}

pub type LlmResult<T> = Result<T, LlmError>;

/// Token usage statistics from an LLM call.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmUsage {
    /// Number of input (prompt) tokens.
    pub input_tokens: u32,
    /// Number of output (completion) tokens.
    pub output_tokens: u32,
    /// Total cost in USD.
    pub cost_usd: f64,
    /// Latency in milliseconds.
    pub latency_ms: u64,
    /// Model used.
    pub model: String,
}

impl LlmUsage {
    /// Total tokens.
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// Configuration for a single LLM completion request.
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    /// System prompt.
    pub system_prompt: Option<String>,
    /// User prompt.
    pub user_prompt: String,
    /// Temperature (0.0 = deterministic, 1.0 = creative).
    pub temperature: f32,
    /// Maximum tokens to generate.
    pub max_tokens: Option<u32>,
    /// Whether to request JSON output.
    pub json_mode: bool,
}

impl CompletionRequest {
    /// Create a simple completion request.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            system_prompt: None,
            user_prompt: prompt.into(),
            temperature: 0.0,
            max_tokens: None,
            json_mode: false,
        }
    }

    /// Set the system prompt.
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system_prompt = Some(system.into());
        self
    }

    /// Enable JSON mode.
    pub fn with_json_mode(mut self) -> Self {
        self.json_mode = true;
        self
    }

    /// Set temperature.
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = temp;
        self
    }

    /// Set max tokens.
    pub fn with_max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }
}

/// Response from an LLM completion.
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    /// The generated text.
    pub content: String,
    /// Token usage statistics.
    pub usage: LlmUsage,
}

/// The core LLM client trait.
///
/// All LLM providers (OpenAI, Anthropic, Ollama, etc.) implement this trait.
/// This enables provider-agnostic LLM usage throughout the engine.
///
/// Note: This trait is dyn-compatible (object-safe). For structured JSON
/// deserialization, use the free function `complete_structured()` below.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Get the model name.
    fn model_name(&self) -> &str;

    /// Get cost per token (input_cost, output_cost) in USD.
    fn cost_per_token(&self) -> (f64, f64);

    /// Which provider this client belongs to.
    fn provider(&self) -> LlmProvider;

    /// What this model can do.
    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities::default()
    }

    /// Estimate cost for given token counts.
    fn estimate_cost(&self, prompt_tokens: u32, completion_tokens: u32) -> f64 {
        let (input_cost, output_cost) = self.cost_per_token();
        prompt_tokens as f64 * input_cost + completion_tokens as f64 * output_cost
    }

    /// Check if the provider is reachable and the API key is valid.
    async fn health_check(&self) -> LlmResult<ProviderHealth> {
        // Default: try a minimal completion to verify connectivity.
        let start = std::time::Instant::now();
        let req = CompletionRequest::new("Say 'ok'")
            .with_max_tokens(5)
            .with_temperature(0.0);
        match self.complete(req).await {
            Ok(_) => Ok(ProviderHealth {
                provider: self.provider(),
                healthy: true,
                latency_ms: start.elapsed().as_millis() as u64,
                available_models: vec![self.model_name().to_string()],
                error: None,
                checked_at: chrono::Utc::now(),
            }),
            Err(e) => Ok(ProviderHealth {
                provider: self.provider(),
                healthy: false,
                latency_ms: start.elapsed().as_millis() as u64,
                available_models: vec![],
                error: Some(e.to_string()),
                checked_at: chrono::Utc::now(),
            }),
        }
    }

    /// Generate a completion.
    async fn complete(&self, request: CompletionRequest) -> LlmResult<CompletionResponse>;
}

/// Generate a structured (JSON) completion and deserialize to type `T`.
///
/// This is a free function rather than a trait method to keep `LlmClient`
/// dyn-compatible (object-safe). Generic type parameters on trait methods
/// prevent the trait from being used as `dyn LlmClient`.
pub async fn complete_structured<T: for<'de> Deserialize<'de> + Send>(
    client: &dyn LlmClient,
    request: CompletionRequest,
) -> LlmResult<(T, LlmUsage)> {
    let request = CompletionRequest {
        json_mode: true,
        ..request
    };
    let response = client.complete(request).await?;
    let value: T = serde_json::from_str(&response.content)
        .map_err(|e| LlmError::DeserializationError(format!("{}: {}", e, response.content)))?;
    Ok((value, response.usage))
}
