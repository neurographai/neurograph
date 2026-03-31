// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Dimension alignment utilities for embedding model switching.
//!
//! When switching between embedding models (e.g., from OpenAI 1536d to
//! Gemini 768d), existing embeddings need alignment. This module provides
//! utilities for padding, truncation, and metadata tracking.

use serde::{Deserialize, Serialize};

/// Handles dimension mismatches when switching embedding models.
pub struct DimensionAligner;

impl DimensionAligner {
    /// Pad or truncate an embedding to match target dimensions.
    ///
    /// - If embedding is shorter: zero-pad to target length
    /// - If embedding is longer: truncate (valid for Matryoshka models)
    /// - If equal: no-op clone
    pub fn align(embedding: &[f32], target_dims: usize) -> Vec<f32> {
        match embedding.len().cmp(&target_dims) {
            std::cmp::Ordering::Equal => embedding.to_vec(),
            std::cmp::Ordering::Less => {
                let mut aligned = embedding.to_vec();
                aligned.resize(target_dims, 0.0);
                aligned
            }
            std::cmp::Ordering::Greater => embedding[..target_dims].to_vec(),
        }
    }

    /// Check if two embeddings have compatible dimensions.
    pub fn compatible(a: &[f32], b: &[f32]) -> bool {
        a.len() == b.len()
    }

    /// L2-normalize an embedding in place.
    pub fn normalize(embedding: &mut [f32]) {
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in embedding.iter_mut() {
                *val /= norm;
            }
        }
    }

    /// Compute cosine similarity between two embeddings.
    /// Returns `None` if dimensions don't match.
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Option<f32> {
        if a.len() != b.len() {
            return None;
        }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        let denom = norm_a * norm_b;
        if denom > 0.0 {
            Some(dot / denom)
        } else {
            Some(0.0)
        }
    }
}

/// Metadata about which model generated an embedding.
///
/// Store alongside embeddings in the graph so you know which
/// model generated each one (important for model migration).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingMetadata {
    /// Model identifier (e.g., "text-embedding-3-small").
    pub model: String,
    /// Output dimensions.
    pub dimensions: usize,
    /// Provider name.
    pub provider: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_pad() {
        let emb = vec![1.0, 2.0, 3.0];
        let aligned = DimensionAligner::align(&emb, 5);
        assert_eq!(aligned, vec![1.0, 2.0, 3.0, 0.0, 0.0]);
    }

    #[test]
    fn test_align_truncate() {
        let emb = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let aligned = DimensionAligner::align(&emb, 3);
        assert_eq!(aligned, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_align_equal() {
        let emb = vec![1.0, 2.0, 3.0];
        let aligned = DimensionAligner::align(&emb, 3);
        assert_eq!(aligned, emb);
    }

    #[test]
    fn test_compatible() {
        assert!(DimensionAligner::compatible(&[1.0, 2.0], &[3.0, 4.0]));
        assert!(!DimensionAligner::compatible(&[1.0], &[3.0, 4.0]));
    }

    #[test]
    fn test_normalize() {
        let mut emb = vec![3.0, 4.0];
        DimensionAligner::normalize(&mut emb);
        let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let sim = DimensionAligner::cosine_similarity(&a, &a).unwrap();
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = DimensionAligner::cosine_similarity(&a, &b).unwrap();
        assert!(sim.abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_dim_mismatch() {
        assert!(DimensionAligner::cosine_similarity(&[1.0], &[1.0, 2.0]).is_none());
    }
}
