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

    // ── New provider factory methods ─────────────────────────────────

    /// Create config for Anthropic Claude Sonnet 4.5 (best balance).
    pub fn anthropic_sonnet() -> Self {
        Self {
            provider: "anthropic".into(),
            model: "claude-sonnet-4-5-20250514".into(),
            api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            base_url: Some("https://api.anthropic.com/v1".into()),
            temperature: 0.0,
            max_tokens: 4096,
            input_cost_per_million: 3.00,
            output_cost_per_million: 15.00,
            timeout_secs: 120,
            max_concurrent: 5,
        }
    }

    /// Create config for Anthropic Claude Haiku 3.5 (fast + cheap).
    pub fn anthropic_haiku() -> Self {
        Self {
            provider: "anthropic".into(),
            model: "claude-3-5-haiku-20241022".into(),
            api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            base_url: Some("https://api.anthropic.com/v1".into()),
            temperature: 0.0,
            max_tokens: 4096,
            input_cost_per_million: 0.80,
            output_cost_per_million: 4.00,
            timeout_secs: 60,
            max_concurrent: 10,
        }
    }

    /// Create config for Google Gemini 2.5 Flash (huge context, cheap).
    pub fn gemini_flash() -> Self {
        Self {
            provider: "gemini".into(),
            model: "gemini-2.5-flash-preview-05-20".into(),
            api_key: std::env::var("GEMINI_API_KEY").ok(),
            base_url: Some("https://generativelanguage.googleapis.com/v1beta".into()),
            temperature: 0.0,
            max_tokens: 8192,
            input_cost_per_million: 0.075,
            output_cost_per_million: 0.30,
            timeout_secs: 120,
            max_concurrent: 10,
        }
    }

    /// Create config for Google Gemini 2.5 Pro (1M context).
    pub fn gemini_pro() -> Self {
        Self {
            provider: "gemini".into(),
            model: "gemini-2.5-pro-preview-06-05".into(),
            api_key: std::env::var("GEMINI_API_KEY").ok(),
            base_url: Some("https://generativelanguage.googleapis.com/v1beta".into()),
            temperature: 0.0,
            max_tokens: 8192,
            input_cost_per_million: 1.25,
            output_cost_per_million: 10.00,
            timeout_secs: 180,
            max_concurrent: 3,
        }
    }

    /// Create config for Groq Llama 3.3 70B (~500 t/s inference).
    pub fn groq_llama() -> Self {
        Self {
            provider: "groq".into(),
            model: "llama-3.3-70b-versatile".into(),
            api_key: std::env::var("GROQ_API_KEY").ok(),
            base_url: Some("https://api.groq.com/openai/v1".into()),
            temperature: 0.0,
            max_tokens: 4096,
            input_cost_per_million: 0.59,
            output_cost_per_million: 0.79,
            timeout_secs: 30,
            max_concurrent: 10,
        }
    }

    /// Create config for xAI Grok 3 Mini (fast + real-time knowledge).
    pub fn xai_grok_mini() -> Self {
        Self {
            provider: "xai".into(),
            model: "grok-3-mini".into(),
            api_key: std::env::var("XAI_API_KEY").ok(),
            base_url: Some("https://api.x.ai/v1".into()),
            temperature: 0.0,
            max_tokens: 4096,
            input_cost_per_million: 0.30,
            output_cost_per_million: 0.50,
            timeout_secs: 60,
            max_concurrent: 5,
        }
    }

    /// Create config from provider name + model string.
    pub fn for_provider(provider: &str, model: &str) -> Self {
        match provider {
            "openai" => {
                let mut cfg = if model.contains("mini") {
                    Self::gpt4o_mini()
                } else {
                    Self::gpt4o()
                };
                cfg.model = model.to_string();
                cfg
            }
            "anthropic" => {
                let mut cfg = if model.contains("haiku") {
                    Self::anthropic_haiku()
                } else {
                    Self::anthropic_sonnet()
                };
                cfg.model = model.to_string();
                cfg
            }
            "gemini" | "google" => {
                let mut cfg = if model.contains("pro") {
                    Self::gemini_pro()
                } else {
                    Self::gemini_flash()
                };
                cfg.model = model.to_string();
                cfg
            }
            "groq" => {
                let mut cfg = Self::groq_llama();
                cfg.model = model.to_string();
                cfg
            }
            "xai" | "grok" => {
                let mut cfg = Self::xai_grok_mini();
                cfg.model = model.to_string();
                cfg
            }
            "ollama" => Self::ollama(model),
            _ => Self::custom(model, "https://api.openai.com/v1", None),
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
