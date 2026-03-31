// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Embedding provider abstraction.

pub mod fastembed;
pub mod openai;
pub mod traits;

// Research Paper Intelligence additions
pub mod ollama;
pub mod cache;
pub mod router;

pub use traits::{Embedder, EmbedderError, EmbedderResult};
pub use ollama::OllamaEmbedder;
pub use cache::EmbeddingCache;
pub use router::{parse_spec, EmbedSpec, available_providers, create_cached_ollama};
