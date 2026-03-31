// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! LRU embedding cache with seahash keying.
//!
//! Standalone cache that can wrap any embedding function.
//! Hit rate is tracked for observability.

use lru::LruCache;
use parking_lot::Mutex;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};

/// Thread-safe LRU cache for embedding vectors.
pub struct EmbeddingCache {
    cache: Mutex<LruCache<u64, Vec<f32>>>,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl EmbeddingCache {
    /// Create a new cache with the given capacity.
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap();
        Self {
            cache: Mutex::new(LruCache::new(cap)),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Get a cached embedding, or return None.
    pub fn get(&self, text: &str) -> Option<Vec<f32>> {
        let key = Self::hash_key(text);
        let mut cache = self.cache.lock();
        if let Some(vec) = cache.get(&key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(vec.clone())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Insert an embedding into the cache.
    pub fn insert(&self, text: &str, embedding: Vec<f32>) {
        let key = Self::hash_key(text);
        let mut cache = self.cache.lock();
        cache.put(key, embedding);
    }

    /// Get an embedding from cache, or compute it using the provided async closure.
    pub async fn get_or_insert<F, Fut>(&self, text: &str, compute: F) -> anyhow::Result<Vec<f32>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<Vec<f32>>>,
    {
        if let Some(cached) = self.get(text) {
            return Ok(cached);
        }
        let embedding = compute().await?;
        self.insert(text, embedding.clone());
        Ok(embedding)
    }

    /// Cache hit rate (0.0 to 1.0).
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed) as f64;
        let misses = self.misses.load(Ordering::Relaxed) as f64;
        let total = hits + misses;
        if total == 0.0 { 0.0 } else { hits / total }
    }

    pub fn hit_count(&self) -> u64 { self.hits.load(Ordering::Relaxed) }
    pub fn miss_count(&self) -> u64 { self.misses.load(Ordering::Relaxed) }
    pub fn len(&self) -> usize { self.cache.lock().len() }
    pub fn is_empty(&self) -> bool { self.cache.lock().is_empty() }

    pub fn clear(&self) {
        self.cache.lock().clear();
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
    }

    fn hash_key(text: &str) -> u64 {
        seahash::hash(text.as_bytes())
    }
}

impl Default for EmbeddingCache {
    fn default() -> Self { Self::new(10_000) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        let cache = EmbeddingCache::new(100);
        assert!(cache.is_empty());
        cache.insert("hello", vec![1.0, 2.0, 3.0]);
        assert_eq!(cache.len(), 1);
        let result = cache.get("hello");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_cache_miss() {
        let cache = EmbeddingCache::new(100);
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_cache_hit_rate() {
        let cache = EmbeddingCache::new(100);
        cache.insert("key1", vec![1.0]);
        cache.get("key1"); // hit
        cache.get("key2"); // miss
        cache.get("key1"); // hit
        assert!((cache.hit_rate() - 0.6667).abs() < 0.01);
    }

    #[test]
    fn test_cache_eviction() {
        let cache = EmbeddingCache::new(2);
        cache.insert("a", vec![1.0]);
        cache.insert("b", vec![2.0]);
        cache.insert("c", vec![3.0]);
        assert!(cache.get("a").is_none());
        assert!(cache.get("b").is_some());
        assert!(cache.get("c").is_some());
    }

    #[tokio::test]
    async fn test_get_or_insert() {
        let cache = EmbeddingCache::new(100);
        let result = cache
            .get_or_insert("hello", || async { Ok(vec![1.0, 2.0]) })
            .await
            .unwrap();
        assert_eq!(result, vec![1.0, 2.0]);

        let result2 = cache
            .get_or_insert("hello", || async { panic!("should not be called") })
            .await
            .unwrap();
        assert_eq!(result2, vec![1.0, 2.0]);
    }
}
