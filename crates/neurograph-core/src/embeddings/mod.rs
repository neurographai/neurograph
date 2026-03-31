// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Embedding helpers — re-exports from the `embedders` module.
//!
//! This module provides convenient access to the embedding subsystem
//! including the universal OpenAI-compatible client, provider registry,
//! caching, and dimension alignment utilities.

pub use crate::embedders::cache::EmbeddingCache;
pub use crate::embedders::ollama::OllamaEmbedder;
pub use crate::embedders::router::{
    available_providers, create_cached_ollama, parse_spec, spec_to_config,
    EmbedSpec, EmbeddingConfig, EmbeddingFactory, EmbeddingRouter,
};
pub use crate::embedders::openai_compatible::{
    ApiKeySource, OpenAICompatibleConfig, OpenAICompatibleEmbedder,
};
pub use crate::embedders::providers::EmbeddingRegistry;
pub use crate::embedders::alignment::{DimensionAligner, EmbeddingMetadata};
pub use crate::embedders::traits::{Embedder, EmbedderError, EmbedderResult, EmbeddingModelInfo};
