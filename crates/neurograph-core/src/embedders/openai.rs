// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Legacy OpenAI embeddings client — DEPRECATED.
//!
//! This module exists only for API compatibility. All new code should use
//! `openai_compatible::OpenAICompatibleEmbedder` which handles OpenAI,
//! Gemini, Cohere, Voyage, Jina, Mistral, Azure, and any future
//! OpenAI-compatible API through a single client.
//!
//! Migration:
//! ```rust,no_run
//! // Old:
//! // let embedder = OpenAiEmbedder::new("text-embedding-3-small")?;
//!
//! // New:
//! use neurograph_core::embedders::providers::EmbeddingRegistry;
//! use neurograph_core::embedders::openai_compatible::OpenAICompatibleEmbedder;
//! let config = EmbeddingRegistry::openai_text_embedding_3_small();
//! let embedder = OpenAICompatibleEmbedder::new(config).unwrap();
//! ```

/// Legacy OpenAI embedder — delegates to `OpenAICompatibleEmbedder`.
///
/// **Deprecated**: Use `OpenAICompatibleEmbedder` directly for all
/// OpenAI-format embedding APIs.
#[deprecated(since = "0.2.0", note = "Use OpenAICompatibleEmbedder instead")]
pub type OpenAiEmbedder = super::openai_compatible::OpenAICompatibleEmbedder;
