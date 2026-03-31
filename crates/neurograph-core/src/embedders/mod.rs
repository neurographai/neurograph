// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Universal embedding provider abstraction.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────┐
//! │                  EmbeddingRouter                      │
//! │  (selects provider, handles fallback)                 │
//! ├──────────┬──────────┬──────────┬─────────────────────┤
//! │ OpenAI-  │  Ollama  │  Hash   │  Custom impl of      │
//! │ Compatible│  Legacy │  Local  │  Embedder trait       │
//! │  Client  │  Client  │         │                      │
//! │ -OpenAI  │ -nomic   │         │ -Your own model      │
//! │ -Gemini  │ -mxbai   │         │ -Enterprise API      │
//! │ -Cohere  │ -any     │         │                      │
//! │ -Voyage  │          │         │                      │
//! │ -Jina    │          │         │                      │
//! │ -Mistral │          │         │                      │
//! │ -Azure   │          │         │                      │
//! └──────────┴──────────┴──────────┴─────────────────────┘
//! ```

pub mod alignment;
pub mod cache;
pub mod config_file;
pub mod fastembed;
pub mod hnsw;
pub mod ollama;
pub mod openai_compatible;
pub mod providers;
pub mod router;
pub mod traits;

// Keep old openai.rs for backward compatibility but it's now deprecated
// in favor of openai_compatible.rs which handles all OpenAI-format APIs.
#[deprecated(note = "Use openai_compatible::OpenAICompatibleEmbedder instead")]
pub mod openai;

// ── Core trait re-exports ──
pub use traits::{Embedder, EmbedderError, EmbedderResult, EmbeddingModelInfo};

// ── Provider re-exports ──
pub use fastembed::HashEmbedder;
pub use ollama::OllamaEmbedder;
pub use openai_compatible::{ApiKeySource, OpenAICompatibleConfig, OpenAICompatibleEmbedder};
pub use providers::EmbeddingRegistry;

// ── Router re-exports ──
pub use router::{
    available_providers, create_cached_ollama, parse_spec, spec_to_config,
    EmbedSpec, EmbeddingConfig, EmbeddingFactory, EmbeddingRouter,
};

// ── Utilities ──
pub use alignment::{DimensionAligner, EmbeddingMetadata};
pub use cache::EmbeddingCache;
pub use config_file::build_from_toml;
pub use hnsw::{HnswConfig, HnswIndex};
