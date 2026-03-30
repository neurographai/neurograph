// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Hash-based local embedder for zero-config operation.
//!
//! This is a deterministic pseudo-embedder that generates fixed-dimension
//! vectors from text using cryptographic hashing (BLAKE3).
//!
//! It does NOT produce semantically meaningful embeddings — it's a fallback
//! for when no API key is available. It enables basic deduplication via
//! exact-match detection but not semantic similarity.
//!
//! For production use, configure OpenAI embeddings or install the
//! `fastembed` crate (TODO: Sprint 2).

use async_trait::async_trait;

use super::traits::{Embedder, EmbedderResult};

/// Hash-based local embedder (zero-config fallback).
///
/// Generates deterministic pseudo-embeddings from text content.
/// Same input always produces the same output.
/// Useful for testing and deduplication, not for semantic search.
#[derive(Debug, Clone)]
pub struct HashEmbedder {
    dimensions: usize,
}

impl HashEmbedder {
    /// Create a new hash embedder with the given dimensions.
    pub fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }
}

impl Default for HashEmbedder {
    fn default() -> Self {
        Self::new(384) // Match fastembed's default BAAI/bge-small-en-v1.5 dimensions
    }
}

/// Generate a pseudo-embedding from text using BLAKE3 hash.
/// Expands the 32-byte hash to fill the desired dimensions.
///
/// Each hash byte is mapped to [-1.0, 1.0] range to avoid IEEE 754
/// NaN/Inf values that would occur with raw `f32::from_le_bytes`.
fn hash_to_embedding(text: &str, dimensions: usize) -> Vec<f32> {
    let normalized = text.to_lowercase().trim().to_string();
    let mut embedding = Vec::with_capacity(dimensions);

    // Generate enough hash bytes to fill dimensions (1 byte → 1 dimension)
    let mut i = 0u64;
    while embedding.len() < dimensions {
        let input = format!("{}{}", normalized, i);
        let hash = blake3::hash(input.as_bytes());
        let bytes = hash.as_bytes();

        for &byte in bytes.iter() {
            if embedding.len() >= dimensions {
                break;
            }
            // Map byte [0, 255] to [-1.0, 1.0] — always finite, deterministic
            let val = (byte as f32 / 127.5) - 1.0;
            embedding.push(val);
        }
        i += 1;
    }

    embedding.truncate(dimensions);

    // L2 normalize
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for val in &mut embedding {
            *val /= norm;
        }
    }

    embedding
}

#[async_trait]
impl Embedder for HashEmbedder {
    fn model_name(&self) -> &str {
        "hash-embedder-v1"
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    async fn embed_batch(&self, texts: &[String]) -> EmbedderResult<Vec<Vec<f32>>> {
        Ok(texts
            .iter()
            .map(|text| hash_to_embedding(text, self.dimensions))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hash_embedder_deterministic() {
        let embedder = HashEmbedder::default();

        let emb1 = embedder.embed_one("hello world").await.unwrap();
        let emb2 = embedder.embed_one("hello world").await.unwrap();
        let emb3 = embedder.embed_one("different text").await.unwrap();

        // Same input → same output
        assert_eq!(emb1, emb2);

        // Different input → different output
        assert_ne!(emb1, emb3);

        // Correct dimensions
        assert_eq!(emb1.len(), 384);
    }

    #[tokio::test]
    async fn test_hash_embedder_normalized() {
        let embedder = HashEmbedder::default();
        let emb = embedder.embed_one("test").await.unwrap();

        // Should be approximately L2-normalized (length ≈ 1.0)
        let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_hash_embedder_batch() {
        let embedder = HashEmbedder::default();
        let texts = vec!["hello".to_string(), "world".to_string()];
        let results = embedder.embed_batch(&texts).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_ne!(results[0], results[1]);
    }
}
