// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Embedding helpers — re-exports from the `embedders` module.
//!
//! This module provides convenient access to the embedding cache,
//! router, and Ollama provider. It complements the existing `embedders`
//! module by adding the Research Paper Intelligence extensions.

pub use crate::embedders::cache::EmbeddingCache;
pub use crate::embedders::ollama::OllamaEmbedder;
pub use crate::embedders::router::{
    available_providers, create_cached_ollama, parse_spec, EmbedSpec,
};
