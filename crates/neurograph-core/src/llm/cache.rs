// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! LLM response caching layer.
//!
//! Prevents redundant API calls for identical extraction prompts.
//! Closes a gap where GraphRAG has a modular caching system for
//! LLM responses with factory support.
//!
//! Uses SHA-256 content hashing for cache keys and supports both
//! in-memory caching with LRU eviction and optional TTL expiration.

use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Cache statistics for observability.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Total cache hits.
    pub hits: u64,
    /// Total cache misses.
    pub misses: u64,
    /// Total entries evicted by LRU.
    pub evictions: u64,
}

impl CacheStats {
    /// Cache hit rate as a percentage.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64 * 100.0
        }
    }
}

/// A cached LLM response with metadata.
#[derive(Debug, Clone)]
pub struct CachedResponse {
    /// The LLM response content.
    pub content: String,
    /// Which model produced this response.
    pub model: String,
    /// Input tokens consumed (for tracking).
    pub input_tokens: u64,
    /// Output tokens consumed (for tracking).
    pub output_tokens: u64,
    /// When this entry was cached.
    pub cached_at: DateTime<Utc>,
    /// Optional TTL — entries expire after this duration.
    pub ttl: Option<Duration>,
}

impl CachedResponse {
    /// Check if this cached entry has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl {
            Utc::now() - self.cached_at > ttl
        } else {
            false // No TTL = never expires
        }
    }
}

/// LLM response cache with SHA-256 key hashing and LRU eviction.
///
/// Thread-safe via `tokio::sync::RwLock`. Supports configurable
/// max capacity and optional per-entry TTL.
///
/// # Example
///
/// ```rust
/// use neurograph_core::llm::cache::LlmCache;
///
/// let cache = LlmCache::new(1000); // Max 1000 entries
///
/// // Cache key is SHA-256(prompt + model)
/// // let response = cache.get("prompt", "gpt-4o").await;
/// ```
pub struct LlmCache {
    /// In-memory cache: SHA256(prompt+model) → response.
    store: RwLock<HashMap<String, CachedResponse>>,
    /// Max entries before LRU eviction.
    max_entries: usize,
    /// Cache statistics.
    stats: RwLock<CacheStats>,
}

impl LlmCache {
    /// Create a new cache with the given maximum capacity.
    pub fn new(max_entries: usize) -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
            max_entries: max_entries.max(1),
            stats: RwLock::new(CacheStats::default()),
        }
    }

    /// Generate a deterministic cache key from prompt and model.
    pub fn cache_key(prompt: &str, model: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prompt.as_bytes());
        hasher.update(b"|");
        hasher.update(model.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Look up a cached response.
    ///
    /// Returns `None` if not found or expired.
    pub async fn get(&self, prompt: &str, model: &str) -> Option<CachedResponse> {
        let key = Self::cache_key(prompt, model);
        let store = self.store.read().await;

        if let Some(entry) = store.get(&key) {
            if entry.is_expired() {
                // Expired — treat as miss (will be evicted on next write)
                self.stats.write().await.misses += 1;
                return None;
            }
            self.stats.write().await.hits += 1;
            Some(entry.clone())
        } else {
            self.stats.write().await.misses += 1;
            None
        }
    }

    /// Store a response in the cache.
    ///
    /// Performs LRU eviction if the cache is at capacity.
    pub async fn put(&self, prompt: &str, model: &str, response: CachedResponse) {
        let key = Self::cache_key(prompt, model);
        let mut store = self.store.write().await;

        // Evict expired entries first
        store.retain(|_, v| !v.is_expired());

        // LRU eviction if still at capacity
        while store.len() >= self.max_entries {
            if let Some(oldest_key) = store
                .iter()
                .min_by_key(|(_, v)| v.cached_at)
                .map(|(k, _)| k.clone())
            {
                store.remove(&oldest_key);
                self.stats.write().await.evictions += 1;
            } else {
                break;
            }
        }

        store.insert(key, response);
    }

    /// Get current cache statistics.
    pub async fn stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// Get the current number of entries.
    pub async fn len(&self) -> usize {
        self.store.read().await.len()
    }

    /// Check if the cache is empty.
    pub async fn is_empty(&self) -> bool {
        self.store.read().await.is_empty()
    }

    /// Clear all cached entries.
    pub async fn clear(&self) {
        self.store.write().await.clear();
    }

    /// Invalidate a specific cache entry.
    pub async fn invalidate(&self, prompt: &str, model: &str) -> bool {
        let key = Self::cache_key(prompt, model);
        self.store.write().await.remove(&key).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_response(content: &str) -> CachedResponse {
        CachedResponse {
            content: content.to_string(),
            model: "gpt-4o".to_string(),
            input_tokens: 100,
            output_tokens: 50,
            cached_at: Utc::now(),
            ttl: None,
        }
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let cache = LlmCache::new(100);
        let response = make_response("Hello world");

        cache.put("test prompt", "gpt-4o", response.clone()).await;
        let cached = cache.get("test prompt", "gpt-4o").await;

        assert!(cached.is_some());
        assert_eq!(cached.unwrap().content, "Hello world");

        let stats = cache.stats().await;
        assert_eq!(stats.hits, 1);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = LlmCache::new(100);
        let cached = cache.get("nonexistent", "gpt-4o").await;
        assert!(cached.is_none());

        let stats = cache.stats().await;
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_cache_key_deterministic() {
        let k1 = LlmCache::cache_key("prompt", "model");
        let k2 = LlmCache::cache_key("prompt", "model");
        assert_eq!(k1, k2);

        let k3 = LlmCache::cache_key("different", "model");
        assert_ne!(k1, k3);
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let cache = LlmCache::new(2);

        cache.put("p1", "m", make_response("r1")).await;
        cache.put("p2", "m", make_response("r2")).await;
        cache.put("p3", "m", make_response("r3")).await; // Should evict p1

        assert!(cache.get("p1", "m").await.is_none());
        assert!(cache.get("p3", "m").await.is_some());
    }

    #[tokio::test]
    async fn test_ttl_expiration() {
        let cache = LlmCache::new(100);

        let expired = CachedResponse {
            content: "old".to_string(),
            model: "gpt-4o".to_string(),
            input_tokens: 100,
            output_tokens: 50,
            cached_at: Utc::now() - Duration::hours(2),
            ttl: Some(Duration::hours(1)),
        };

        cache.put("old_prompt", "gpt-4o", expired).await;
        assert!(cache.get("old_prompt", "gpt-4o").await.is_none());
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = LlmCache::new(100);
        cache.put("p1", "m", make_response("r1")).await;
        assert!(!cache.is_empty().await);

        cache.clear().await;
        assert!(cache.is_empty().await);
    }

    #[tokio::test]
    async fn test_invalidate() {
        let cache = LlmCache::new(100);
        cache.put("p1", "m", make_response("r1")).await;

        assert!(cache.invalidate("p1", "m").await);
        assert!(!cache.invalidate("nonexistent", "m").await);
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let stats = CacheStats {
            hits: 80,
            misses: 20,
            evictions: 5,
        };
        assert!((stats.hit_rate() - 80.0).abs() < f64::EPSILON);
    }
}
