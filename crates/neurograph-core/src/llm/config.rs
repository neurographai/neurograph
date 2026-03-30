// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! LLM configuration and model definitions.

use serde::{Deserialize, Serialize};

/// Configuration for an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Provider name (e.g., "openai", "anthropic", "ollama").
    pub provider: String,
    /// Model name (e.g., "gpt-4o-mini", "claude-3-haiku").
    pub model: String,
    /// API key (can also be set via env var).
    pub api_key: Option<String>,
    /// API base URL (for custom endpoints / Ollama / vLLM).
    pub base_url: Option<String>,
    /// Default temperature.
    pub temperature: f32,
    /// Max tokens per completion.
    pub max_tokens: u32,
    /// Cost per 1M input tokens in USD.
    pub input_cost_per_million: f64,
    /// Cost per 1M output tokens in USD.
    pub output_cost_per_million: f64,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
    /// Maximum concurrent requests.
    pub max_concurrent: usize,
}

impl LlmConfig {
    /// Create config for OpenAI GPT-4o-mini (cost-effective default).
    pub fn gpt4o_mini() -> Self {
        Self {
            provider: "openai".into(),
            model: "gpt-4o-mini".into(),
            api_key: std::env::var("OPENAI_API_KEY").ok(),
            base_url: None,
            temperature: 0.0,
            max_tokens: 4096,
            input_cost_per_million: 0.15,
            output_cost_per_million: 0.60,
            timeout_secs: 60,
            max_concurrent: 10,
        }
    }

    /// Create config for OpenAI GPT-4o (higher quality).
    pub fn gpt4o() -> Self {
        Self {
            provider: "openai".into(),
            model: "gpt-4o".into(),
            api_key: std::env::var("OPENAI_API_KEY").ok(),
            base_url: None,
            temperature: 0.0,
            max_tokens: 4096,
            input_cost_per_million: 2.50,
            output_cost_per_million: 10.00,
            timeout_secs: 120,
            max_concurrent: 5,
        }
    }

    /// Create config for Ollama (local, free).
    pub fn ollama(model: impl Into<String>) -> Self {
        Self {
            provider: "ollama".into(),
            model: model.into(),
            api_key: None,
            base_url: Some("http://localhost:11434/v1".into()),
            temperature: 0.0,
            max_tokens: 4096,
            input_cost_per_million: 0.0,
            output_cost_per_million: 0.0,
            timeout_secs: 300,
            max_concurrent: 2,
        }
    }

    /// Create config for any OpenAI-compatible API.
    pub fn custom(
        model: impl Into<String>,
        base_url: impl Into<String>,
        api_key: Option<String>,
    ) -> Self {
        Self {
            provider: "custom".into(),
            model: model.into(),
            api_key,
            base_url: Some(base_url.into()),
            temperature: 0.0,
            max_tokens: 4096,
            input_cost_per_million: 0.0,
            output_cost_per_million: 0.0,
            timeout_secs: 120,
            max_concurrent: 5,
        }
    }

    /// Calculate cost for a given number of tokens.
    pub fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.input_cost_per_million;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_cost_per_million;
        input_cost + output_cost
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self::gpt4o_mini()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_calculation() {
        let config = LlmConfig::gpt4o_mini();
        let cost = config.calculate_cost(1000, 500);
        // 1000 input tokens * $0.15/1M = $0.00015
        // 500 output tokens * $0.60/1M = $0.0003
        assert!((cost - 0.00045).abs() < 0.0001);
    }
}
