// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Deterministic hashing utilities for entity deduplication.

use sha2::{Digest, Sha256};

/// Generate a deterministic content hash for deduplication.
///
/// Used in the two-phase dedup strategy (from Graphiti):
/// 1. Phase 1: Deterministic hash matching (this function)
/// 2. Phase 2: Embedding similarity + LLM fallback
pub fn content_hash(content: &str) -> String {
    let normalized = normalize_for_hash(content);
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Generate a BLAKE3 hash (faster, for high-throughput scenarios).
pub fn fast_hash(content: &str) -> String {
    let normalized = normalize_for_hash(content);
    blake3::hash(normalized.as_bytes()).to_hex().to_string()
}

/// Normalize text for consistent hashing.
/// Lowercases, trims whitespace, collapses multiple spaces.
fn normalize_for_hash(text: &str) -> String {
    text.to_lowercase()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Generate a deterministic entity dedup key from name + type.
pub fn entity_dedup_key(name: &str, entity_type: &str) -> String {
    let input = format!(
        "{}::{}",
        name.to_lowercase().trim(),
        entity_type.to_lowercase().trim()
    );
    content_hash(&input)
}

/// Generate a deterministic relationship dedup key.
pub fn relationship_dedup_key(source_name: &str, target_name: &str, rel_type: &str) -> String {
    let input = format!(
        "{}::{}::{}",
        source_name.to_lowercase().trim(),
        rel_type.to_lowercase().trim(),
        target_name.to_lowercase().trim()
    );
    content_hash(&input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash_deterministic() {
        let h1 = content_hash("Hello World");
        let h2 = content_hash("Hello World");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_content_hash_normalization() {
        let h1 = content_hash("  Hello   World  ");
        let h2 = content_hash("hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_entity_dedup_key() {
        let k1 = entity_dedup_key("Alice", "Person");
        let k2 = entity_dedup_key("alice", "person");
        let k3 = entity_dedup_key("Bob", "Person");
        assert_eq!(k1, k2); // Same entity, different case
        assert_ne!(k1, k3); // Different entities
    }

    #[test]
    fn test_fast_hash() {
        let h1 = fast_hash("test");
        let h2 = fast_hash("test");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // BLAKE3 hex = 64 chars
    }
}
