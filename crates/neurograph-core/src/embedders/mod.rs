// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Embedding provider abstraction.

pub mod fastembed;
pub mod openai;
pub mod traits;

pub use traits::{Embedder, EmbedderError, EmbedderResult};
