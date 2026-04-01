// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! LLM provider implementations.
//!
//! Each provider implements the `LlmClient` trait from `super::traits`.
//! - `anthropic.rs` — Anthropic Claude (custom API format)
//! - `gemini.rs` — Google Gemini (custom API format)
//! - `openai_compat.rs` — OpenAI-compatible (xAI Grok, Groq)

pub mod anthropic;
pub mod gemini;
pub mod openai_compat;

pub use anthropic::AnthropicClient;
pub use gemini::GeminiClient;
pub use openai_compat::OpenAiCompatClient;
