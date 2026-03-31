// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! HNSW (Hierarchical Navigable Small World) index for approximate
//! nearest neighbor search.
//!
//! Replaces brute-force O(n) cosine scan with O(log n) approximate
//! nearest neighbor queries. Based on Malkov & Yashunin (2018).
//!
//! # Usage
//!
//! ```rust
//! use neurograph_core::embedders::hnsw::{HnswIndex, HnswConfig};
//!
//! let index = HnswIndex::new(HnswConfig::default());
//! index.insert("doc-1".into(), vec![1.0, 0.0, 0.0]);
//! index.insert("doc-2".into(), vec![0.9, 0.1, 0.0]);
//!
//! let results = index.search(&[1.0, 0.0, 0.0], 2);
//! assert_eq!(results[0].0, "doc-1");
//! ```

use parking_lot::RwLock;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Reverse;

// ── Configuration ────────────────────────────────────────────────────────

/// Configuration for the HNSW index.
#[derive(Debug, Clone)]
pub struct HnswConfig {
    /// Max connections per node per layer (M in the paper).
    pub max_connections: usize,
    /// Max connections for layer 0 (M0 = 2*M).
    pub max_connections_0: usize,
    /// Size of dynamic candidate list during construction (ef_construction).
    pub ef_construction: usize,
    /// Size of dynamic candidate list during search (ef_search).
    pub ef_search: usize,
    /// Level generation factor (mL = 1/ln(M)).
    pub level_multiplier: f64,
}

impl Default for HnswConfig {
    fn default() -> Self {
        Self {
            max_connections: 16,
            max_connections_0: 32,
            ef_construction: 200,
            ef_search: 50,
            level_multiplier: 1.0 / (16.0_f64).ln(),
        }
    }
}

// ── Ordered float wrapper ────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
struct OrdF32(f32);

impl Eq for OrdF32 {}

impl PartialOrd for OrdF32 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrdF32 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(std::cmp::Ordering::Equal)
    }
}

// ── HNSW Index ───────────────────────────────────────────────────────────

/// Thread-safe HNSW index for approximate nearest neighbor search.
///
/// Provides O(log n) insert and query operations vs O(n) brute force.
pub struct HnswIndex {
    /// All stored vectors keyed by ID.
    vectors: RwLock<HashMap<String, Vec<f32>>>,
    /// Adjacency lists per layer: `layers[l][node_id] = [neighbor_ids]`.
    layers: RwLock<Vec<HashMap<String, Vec<String>>>>,
    /// Entry point node (exists on the highest layer).
    entry_point: RwLock<Option<String>>,
    /// Maximum layer of the entry point.
    max_layer: RwLock<usize>,
    /// Configuration.
    config: HnswConfig,
}

impl std::fmt::Debug for HnswIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HnswIndex")
            .field("size", &self.vectors.read().len())
            .field("layers", &self.layers.read().len())
            .field("config", &self.config)
            .finish()
    }
}

impl HnswIndex {
    /// Create a new empty HNSW index.
    pub fn new(config: HnswConfig) -> Self {
        Self {
            vectors: RwLock::new(HashMap::new()),
            layers: RwLock::new(vec![HashMap::new()]),
            entry_point: RwLock::new(None),
            max_layer: RwLock::new(0),
            config,
        }
    }

    /// Insert a vector into the index. O(log n) amortized.
    pub fn insert(&self, id: String, vector: Vec<f32>) {
        let new_level = self.random_level();

        // Store the vector
        self.vectors.write().insert(id.clone(), vector.clone());

        // Ensure we have enough layers
        {
            let mut layers = self.layers.write();
            while layers.len() <= new_level {
                layers.push(HashMap::new());
            }
        }

        let entry = self.entry_point.read().clone();

        if entry.is_none() {
            // First node — becomes the entry point
            self.layers.write()[0].insert(id.clone(), Vec::new());
            for l in 1..=new_level {
                self.layers.write()[l].insert(id.clone(), Vec::new());
            }
            *self.entry_point.write() = Some(id);
            *self.max_layer.write() = new_level;
            return;
        }

        let entry_id = entry.unwrap();
        let current_max_layer = *self.max_layer.read();
        let mut current_best = entry_id;

        // Phase 1: Greedy descent from top layer down to new_level + 1
        // (find the closest node to our insert point at each layer)
        let top = current_max_layer.min(self.layers.read().len().saturating_sub(1));
        for l in (new_level + 1..=top).rev() {
            current_best = self.greedy_closest(&vector, &current_best, l);
        }

        // Phase 2: Insert at layers [min(new_level, top)..=0]
        let insert_top = new_level.min(self.layers.read().len().saturating_sub(1));
        for l in (0..=insert_top).rev() {
            let candidates = self.search_layer(&vector, &current_best, self.config.ef_construction, l);

            let max_conn = if l == 0 {
                self.config.max_connections_0
            } else {
                self.config.max_connections
            };

            // Select top neighbors by similarity
            let neighbors: Vec<String> = candidates
                .iter()
                .take(max_conn)
                .map(|(nid, _)| nid.clone())
                .collect();

            // Add bidirectional edges
            {
                let mut layers = self.layers.write();
                let layer = &mut layers[l];

                // Add our node with its neighbors
                layer.entry(id.clone()).or_default().extend(neighbors.iter().cloned());

                // Add reverse edges and prune if needed
                for neighbor in &neighbors {
                    let n_neighbors = layer.entry(neighbor.clone()).or_default();
                    n_neighbors.push(id.clone());

                    // Prune if over capacity
                    if n_neighbors.len() > max_conn {
                        let vectors = self.vectors.read();
                        if let Some(nv) = vectors.get(neighbor) {
                            let mut scored: Vec<(String, f32)> = n_neighbors
                                .iter()
                                .filter_map(|nn| {
                                    vectors.get(nn).map(|v| (nn.clone(), cosine_similarity(nv, v)))
                                })
                                .collect();
                            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                            *n_neighbors = scored.into_iter().take(max_conn).map(|(nid, _)| nid).collect();
                        }
                    }
                }
            }

            // Update starting point for next layer down
            if !candidates.is_empty() {
                current_best = candidates[0].0.clone();
            }
        }

        // Update entry point if new node has a higher level
        if new_level > current_max_layer {
            *self.entry_point.write() = Some(id);
            *self.max_layer.write() = new_level;
        }
    }

    /// Search for the k nearest neighbors. O(log n) average case.
    ///
    /// Returns `Vec<(id, similarity)>` sorted by descending similarity.
    pub fn search(&self, query: &[f32], k: usize) -> Vec<(String, f32)> {
        let entry = match self.entry_point.read().clone() {
            Some(e) => e,
            None => return vec![],
        };

        let top_layer = self.max_layer.read().min(self.layers.read().len().saturating_sub(1));
        let mut current = entry;

        // Greedy descent from top layer down to layer 1
        for l in (1..=*&top_layer).rev() {
            current = self.greedy_closest(query, &current, l);
        }

        // Search layer 0 with ef_search candidates, then take top k
        let mut results = self.search_layer(query, &current, self.config.ef_search.max(k), 0);
        results.truncate(k);
        results
    }

    /// Remove a vector by ID.
    pub fn remove(&self, id: &str) {
        self.vectors.write().remove(id);

        let mut layers = self.layers.write();
        for layer in layers.iter_mut() {
            layer.remove(id);
            // Remove from all neighbor lists
            for neighbors in layer.values_mut() {
                neighbors.retain(|n| n != id);
            }
        }

        // If we removed the entry point, pick a new one
        let ep = self.entry_point.read().clone();
        if ep.as_deref() == Some(id) {
            let new_ep = layers
                .iter()
                .rev()
                .find_map(|layer| layer.keys().next().cloned());
            *self.entry_point.write() = new_ep;
        }
    }

    /// Number of indexed vectors.
    pub fn len(&self) -> usize {
        self.vectors.read().len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.vectors.read().is_empty()
    }

    /// Clear all data from the index.
    pub fn clear(&self) {
        self.vectors.write().clear();
        *self.layers.write() = vec![HashMap::new()];
        *self.entry_point.write() = None;
        *self.max_layer.write() = 0;
    }

    // ── Internal helpers ─────────────────────────────────────────────────

    /// Generate a random level for a new node using geometric distribution.
    fn random_level(&self) -> usize {
        let r: f64 = rand::random();
        // Clamp to avoid degenerate cases
        let level = (-r.ln() * self.config.level_multiplier).floor() as usize;
        level.min(20) // cap at 20 layers
    }

    /// Greedily find the closest node to `query` starting from `start` on `layer`.
    fn greedy_closest(&self, query: &[f32], start: &str, layer: usize) -> String {
        let vectors = self.vectors.read();
        let layers = self.layers.read();

        let mut current = start.to_string();
        let mut best_sim = vectors
            .get(start)
            .map(|v| cosine_similarity(query, v))
            .unwrap_or(-1.0);

        loop {
            let mut improved = false;
            let neighbors = layers
                .get(layer)
                .and_then(|l| l.get(&current))
                .cloned()
                .unwrap_or_default();

            for neighbor in &neighbors {
                if let Some(v) = vectors.get(neighbor) {
                    let sim = cosine_similarity(query, v);
                    if sim > best_sim {
                        best_sim = sim;
                        current = neighbor.clone();
                        improved = true;
                    }
                }
            }

            if !improved {
                break;
            }
        }

        current
    }

    /// Search a single layer using beam search, returning candidates sorted
    /// by descending similarity.
    fn search_layer(
        &self,
        query: &[f32],
        start: &str,
        ef: usize,
        layer: usize,
    ) -> Vec<(String, f32)> {
        let vectors = self.vectors.read();
        let layers = self.layers.read();

        let start_sim = vectors
            .get(start)
            .map(|v| cosine_similarity(query, v))
            .unwrap_or(-1.0);

        // candidates: max-heap by similarity (best first to explore)
        let mut candidates: BinaryHeap<(OrdF32, String)> = BinaryHeap::new();
        // results: min-heap by similarity (worst first to evict)
        let mut results: BinaryHeap<Reverse<(OrdF32, String)>> = BinaryHeap::new();
        let mut visited: HashSet<String> = HashSet::new();

        candidates.push((OrdF32(start_sim), start.to_string()));
        results.push(Reverse((OrdF32(start_sim), start.to_string())));
        visited.insert(start.to_string());

        while let Some((OrdF32(c_sim), c_id)) = candidates.pop() {
            // If the best candidate is worse than the worst result and we have enough, stop
            let worst_result = results
                .peek()
                .map(|Reverse((OrdF32(s), _))| *s)
                .unwrap_or(-1.0);

            if c_sim < worst_result && results.len() >= ef {
                break;
            }

            let neighbors = layers
                .get(layer)
                .and_then(|l| l.get(&c_id))
                .cloned()
                .unwrap_or_default();

            for neighbor in neighbors {
                if visited.contains(&neighbor) {
                    continue;
                }
                visited.insert(neighbor.clone());

                if let Some(v) = vectors.get(&neighbor) {
                    let sim = cosine_similarity(query, v);
                    let worst = results
                        .peek()
                        .map(|Reverse((OrdF32(s), _))| *s)
                        .unwrap_or(-1.0);

                    if sim > worst || results.len() < ef {
                        candidates.push((OrdF32(sim), neighbor.clone()));
                        results.push(Reverse((OrdF32(sim), neighbor)));

                        if results.len() > ef {
                            results.pop();
                        }
                    }
                }
            }
        }

        // Collect and sort by descending similarity
        let mut result_vec: Vec<(String, f32)> = results
            .into_iter()
            .map(|Reverse((OrdF32(s), id))| (id, s))
            .collect();
        result_vec.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        result_vec
    }
}

// ── Cosine Similarity ────────────────────────────────────────────────────

/// Compute cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_index() {
        let index = HnswIndex::new(HnswConfig::default());
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
        let results = index.search(&[1.0, 0.0], 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_single_insert() {
        let index = HnswIndex::new(HnswConfig::default());
        index.insert("a".into(), vec![1.0, 0.0, 0.0]);
        assert_eq!(index.len(), 1);

        let results = index.search(&[1.0, 0.0, 0.0], 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "a");
        assert!((results[0].1 - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_insert_and_search() {
        let index = HnswIndex::new(HnswConfig::default());

        index.insert("a".into(), vec![1.0, 0.0, 0.0]);
        index.insert("b".into(), vec![0.9, 0.1, 0.0]);
        index.insert("c".into(), vec![0.0, 1.0, 0.0]);
        index.insert("d".into(), vec![0.0, 0.0, 1.0]);

        let results = index.search(&[1.0, 0.0, 0.0], 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "a");
        assert_eq!(results[1].0, "b");
    }

    #[test]
    fn test_search_returns_sorted() {
        let index = HnswIndex::new(HnswConfig::default());

        index.insert("exact".into(), vec![1.0, 0.0, 0.0]);
        index.insert("close".into(), vec![0.95, 0.05, 0.0]);
        index.insert("far".into(), vec![0.0, 0.0, 1.0]);
        index.insert("medium".into(), vec![0.5, 0.5, 0.0]);

        let results = index.search(&[1.0, 0.0, 0.0], 4);
        assert_eq!(results.len(), 4);

        // Must be sorted descending by similarity
        for i in 1..results.len() {
            assert!(
                results[i - 1].1 >= results[i].1,
                "Results not sorted at index {}: {} < {}",
                i,
                results[i - 1].1,
                results[i].1,
            );
        }
    }

    #[test]
    fn test_remove() {
        let index = HnswIndex::new(HnswConfig::default());

        index.insert("a".into(), vec![1.0, 0.0, 0.0]);
        index.insert("b".into(), vec![0.0, 1.0, 0.0]);
        assert_eq!(index.len(), 2);

        index.remove("a");
        assert_eq!(index.len(), 1);

        let results = index.search(&[1.0, 0.0, 0.0], 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "b");
    }

    #[test]
    fn test_clear() {
        let index = HnswIndex::new(HnswConfig::default());
        index.insert("a".into(), vec![1.0, 0.0]);
        index.insert("b".into(), vec![0.0, 1.0]);
        assert_eq!(index.len(), 2);

        index.clear();
        assert!(index.is_empty());
        assert!(index.search(&[1.0, 0.0], 5).is_empty());

        // Can insert again after clear
        index.insert("c".into(), vec![1.0, 0.0]);
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_100_vectors() {
        let config = HnswConfig {
            max_connections: 8,
            max_connections_0: 16,
            ef_construction: 50,
            ef_search: 30,
            ..Default::default()
        };
        let index = HnswIndex::new(config);

        // Insert 100 pseudo-random vectors of dimension 32
        for i in 0..100 {
            let v: Vec<f32> = (0..32)
                .map(|j| ((i * 7 + j * 13 + 5) % 100) as f32 / 100.0)
                .collect();
            index.insert(format!("v{}", i), v);
        }

        assert_eq!(index.len(), 100);

        let query: Vec<f32> = (0..32).map(|j| (j % 100) as f32 / 100.0).collect();
        let results = index.search(&query, 10);
        assert_eq!(results.len(), 10);

        // Results must be sorted by descending similarity
        for i in 1..results.len() {
            assert!(results[i - 1].1 >= results[i].1);
        }
    }

    #[test]
    fn test_1000_vectors_recall() {
        // Verify HNSW achieves reasonable recall vs brute force
        let config = HnswConfig {
            max_connections: 32,
            max_connections_0: 64,
            ef_construction: 200,
            ef_search: 200, // higher ef for better recall
            ..Default::default()
        };
        let index = HnswIndex::new(config);

        let mut all_vectors: Vec<(String, Vec<f32>)> = Vec::new();

        for i in 0..1000 {
            let v: Vec<f32> = (0..64)
                .map(|j| ((i * 17 + j * 31 + 7) % 256) as f32 / 255.0)
                .collect();
            let id = format!("v{}", i);
            all_vectors.push((id.clone(), v.clone()));
            index.insert(id, v);
        }

        let query: Vec<f32> = (0..64).map(|j| (j * 4 % 256) as f32 / 255.0).collect();

        // Brute-force ground truth
        let mut brute_force: Vec<(String, f32)> = all_vectors
            .iter()
            .map(|(id, v)| (id.clone(), cosine_similarity(&query, v)))
            .collect();
        brute_force.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let gt_top10: HashSet<String> = brute_force.iter().take(10).map(|(id, _)| id.clone()).collect();

        // HNSW search
        let hnsw_results = index.search(&query, 10);
        let hnsw_top10: HashSet<String> = hnsw_results.iter().map(|(id, _)| id.clone()).collect();

        // Recall should be >= 60% (pseudo-random vectors lack natural clustering, 
        // real embeddings typically see >90% recall)
        let recall = gt_top10.intersection(&hnsw_top10).count() as f64 / 10.0;
        assert!(
            recall >= 0.6,
            "HNSW recall too low: {:.0}% (expected >= 60%)",
            recall * 100.0,
        );
    }

    #[test]
    fn test_cosine_similarity_fn() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 0.001);

        let d = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &d) + 1.0).abs() < 0.001);
    }
}
