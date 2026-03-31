// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! LLM provider abstraction layer.
//!
//! Provides a trait-based interface for LLM interactions with:
//! - Response caching (SHA-256 keyed, LRU eviction)
//! - Per-prompt-type token tracking
//! - Multiple provider support (OpenAI, generic OpenAI-compatible)

pub mod cache;
pub mod config;
pub mod generic;
pub mod openai;
pub mod token_tracker;
pub mod traits;

pub use cache::LlmCache;
pub use config::LlmConfig;
pub use token_tracker::{PromptType, TokenTracker, TokenUsage};
pub use traits::{complete_structured, LlmClient, LlmError, LlmResult, LlmUsage};
