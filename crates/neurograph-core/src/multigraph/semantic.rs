// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Semantic graph view: nodes connected by embedding similarity.

use super::*;
use std::collections::{BinaryHeap, HashMap, HashSet};
use ordered_float::OrderedFloat;

/// Semantic graph: edges represent embedding similarity between memory items.
pub struct SemanticGraph {
    /// Adjacency list: node_id -> Vec<(neighbor_id, similarity)>
    adjacency: HashMap<Uuid, Vec<(Uuid, f64)>>,
    /// Node embeddings for similarity computation
    embeddings: HashMap<Uuid, Vec<f32>>,
    config: MultiGraphConfig,
}

impl SemanticGraph {
    pub fn new(config: &MultiGraphConfig) -> Self {
        Self {
            adjacency: HashMap::new(),
            embeddings: HashMap::new(),
            config: config.clone(),
        }
    }

    /// Add a memory item and create edges to similar existing nodes.
    pub fn add_item(&mut self, item: &MemoryItem) -> Vec<GraphEdge> {
        let mut new_edges = Vec::new();

        // Find top-K similar existing nodes
        let neighbors = self.find_similar(&item.embedding, self.config.max_neighbors);

        for (neighbor_id, similarity) in &neighbors {
            if *similarity >= self.config.similarity_threshold {
                let edge = GraphEdge {
                    id: Uuid::new_v4(),
                    source: item.id,
                    target: *neighbor_id,
                    view: GraphView::Semantic,
                    relation: "semantically_similar".to_string(),
                    weight: *similarity,
                    valid_from: item.valid_from,
                    valid_until: item.valid_until,
                    metadata: HashMap::new(),
                };

                // Bidirectional edges
                self.adjacency
                    .entry(item.id)
                    .or_default()
                    .push((*neighbor_id, *similarity));
                self.adjacency
                    .entry(*neighbor_id)
                    .or_default()
                    .push((item.id, *similarity));

                new_edges.push(edge);
            }
        }

        self.embeddings.insert(item.id, item.embedding.clone());
        new_edges
    }

    /// Traverse the semantic graph using greedy beam search from query embedding.
    pub fn traverse(&self, _query: &str, opts: &QueryOptions) -> SubgraphResult {
        let start = std::time::Instant::now();
        let mut visited: HashSet<Uuid> = HashSet::new();
        let mut results = Vec::new();

        // Use a max-heap (priority queue) for best-first traversal
        let mut heap: BinaryHeap<(OrderedFloat<f64>, Uuid)> = BinaryHeap::new();

        // Seed with all nodes (in production, would use query embedding similarity)
        for (id, _emb) in &self.embeddings {
            heap.push((OrderedFloat(0.5), *id));
        }

        while let Some((score, node_id)) = heap.pop() {
            if visited.contains(&node_id) {
                continue;
            }
            if results.len() >= opts.max_results {
                break;
            }

            visited.insert(node_id);
            results.push((node_id, score.0));

            // Expand neighbors
            if let Some(neighbors) = self.adjacency.get(&node_id) {
                for (neighbor_id, edge_weight) in neighbors {
                    if !visited.contains(neighbor_id) {
                        let propagated = score.0 * edge_weight;
                        heap.push((OrderedFloat(propagated), *neighbor_id));
                    }
                }
            }
        }

        SubgraphResult {
            view: GraphView::Semantic,
            node_ids: results.iter().map(|(id, _)| *id).collect(),
            scores: results.iter().map(|(_, s)| *s).collect(),
            edges: Vec::new(),
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// Number of edges in the semantic graph.
    pub fn edge_count(&self) -> usize {
        self.adjacency.values().map(|v| v.len()).sum::<usize>() / 2
    }

    fn find_similar(&self, query_emb: &[f32], top_k: usize) -> Vec<(Uuid, f64)> {
        let mut scores: Vec<(Uuid, f64)> = self
            .embeddings
            .iter()
            .map(|(id, emb)| (*id, cosine_similarity(query_emb, emb)))
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }
}

/// Cosine similarity between two float vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    (dot / (norm_a * norm_b)) as f64
}
