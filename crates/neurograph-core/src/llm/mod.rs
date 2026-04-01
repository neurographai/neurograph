// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! LLM provider abstraction layer.
//!
//! Provides a trait-based interface for LLM interactions with:
//! - Response caching (SHA-256 keyed, LRU eviction)
//! - Per-prompt-type token tracking
//! - Multiple provider support (OpenAI, Anthropic, Gemini, xAI Grok, Groq, Ollama)
//! - Smart task-aware routing
//! - Static model registry with pricing + capabilities

pub mod cache;
pub mod config;
pub mod generic;
pub mod openai;
pub mod providers;
pub mod registry;
pub mod router;
pub mod token_tracker;
pub mod traits;

pub use cache::LlmCache;
pub use config::LlmConfig;
pub use registry::{TaskType, ModelInfo, get_model_registry, models_for_provider, models_for_task};
pub use router::{LlmRouter, RouterConfig, RoutingStrategy};
pub use token_tracker::{PromptType, TokenTracker, TokenUsage};
pub use traits::{
    complete_structured, LlmClient, LlmError, LlmProvider, LlmResult, LlmUsage,
    ModelCapabilities, ProviderHealth, SpeedTier,
};
