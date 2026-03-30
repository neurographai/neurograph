// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Personalized PageRank for query-time graph traversal.
//!
//! At inference time, PPR seeds teleport probability from query-relevant
//! entities and propagates scores through the graph structure. This finds
//! entities that are structurally close to the query in the graph, even if
//! they don't share embedding similarity.
//!
//! Reference: "An open-source speed-first GraphRAG implementation uses
//! Personalized PageRank at inference time" (2026).

use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Personalized PageRank for query-time graph traversal.
///
/// Unlike global PageRank, PPR biases the random walk toward a set of
/// "seed" nodes (typically the top semantic search results), finding
/// entities that are structurally important relative to the query.
pub struct PersonalizedPageRank {
    /// Damping factor: probability of following an edge vs. teleporting.
    /// Standard value: 0.85.
    alpha: f64,
    /// Maximum power-iteration steps.
    max_iter: usize,
    /// Convergence threshold (L1 norm of score change).
    epsilon: f64,
}

impl PersonalizedPageRank {
    /// Create a PPR instance with custom parameters.
    pub fn new(alpha: f64, max_iter: usize, epsilon: f64) -> Self {
        Self {
            alpha,
            max_iter,
            epsilon,
        }
    }

    /// Create with standard defaults (α=0.85, 100 iterations, ε=1e-8).
    pub fn default_params() -> Self {
        Self::new(0.85, 100, 1e-8)
    }

    /// Compute PPR scores from a set of seed nodes.
    ///
    /// # Arguments
    /// * `adjacency` — Graph as adjacency list: `node_id -> Vec<(neighbor_id, edge_weight)>`
    /// * `seed_nodes` — Seed nodes with teleport probabilities (will be normalized to sum to 1.0)
    ///
    /// # Returns
    /// Map of `node_id -> PPR score` for all reachable nodes.
    pub fn compute(
        &self,
        adjacency: &HashMap<Uuid, Vec<(Uuid, f64)>>,
        seed_nodes: &HashMap<Uuid, f64>,
    ) -> HashMap<Uuid, f64> {
        // Collect all node IDs
        let mut all_nodes: HashSet<Uuid> = HashSet::new();
        for (node, neighbors) in adjacency {
            all_nodes.insert(*node);
            for (neighbor, _) in neighbors {
                all_nodes.insert(*neighbor);
            }
        }
        for node in seed_nodes.keys() {
            all_nodes.insert(*node);
        }

        let n = all_nodes.len();
        if n == 0 {
            return HashMap::new();
        }

        // Index nodes for efficient array access
        let node_list: Vec<Uuid> = all_nodes.into_iter().collect();
        let node_idx: HashMap<Uuid, usize> = node_list
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, i))
            .collect();

        // Build and normalize personalization vector
        let mut personalization = vec![0.0f64; n];
        for (node, prob) in seed_nodes {
            if let Some(&idx) = node_idx.get(node) {
                personalization[idx] = *prob;
            }
        }
        let pers_sum: f64 = personalization.iter().sum();
        if pers_sum > 0.0 {
            for p in &mut personalization {
                *p /= pers_sum;
            }
        } else {
            // Uniform if no seeds have probability
            let uniform = 1.0 / n as f64;
            for p in &mut personalization {
                *p = uniform;
            }
        }

        // Precompute total outgoing weight per node
        let mut out_weights = vec![0.0f64; n];
        for (node, neighbors) in adjacency {
            if let Some(&idx) = node_idx.get(node) {
                out_weights[idx] = neighbors.iter().map(|(_, w)| w).sum();
            }
        }

        // Power iteration
        let mut scores = personalization.clone();
        let mut new_scores = vec![0.0f64; n];

        for _iter in 0..self.max_iter {
            // Reset
            for s in &mut new_scores {
                *s = 0.0;
            }

            // Propagate scores along edges
            for (node, neighbors) in adjacency {
                if let Some(&src_idx) = node_idx.get(node) {
                    if out_weights[src_idx] > 0.0 {
                        for (neighbor, weight) in neighbors {
                            if let Some(&dst_idx) = node_idx.get(neighbor) {
                                new_scores[dst_idx] +=
                                    self.alpha * scores[src_idx] * weight / out_weights[src_idx];
                            }
                        }
                    }
                }
            }

            // Add teleport (personalization)
            for i in 0..n {
                new_scores[i] += (1.0 - self.alpha) * personalization[i];
            }

            // Handle dangling nodes (no outgoing edges) — distribute their
            // mass according to the personalization vector
            let dangling_sum: f64 = (0..n)
                .filter(|&i| out_weights[i] == 0.0)
                .map(|i| scores[i])
                .sum();

            for i in 0..n {
                new_scores[i] += self.alpha * dangling_sum * personalization[i];
            }

            // Check convergence (L1 norm)
            let diff: f64 = scores
                .iter()
                .zip(new_scores.iter())
                .map(|(a, b)| (a - b).abs())
                .sum();

            std::mem::swap(&mut scores, &mut new_scores);

            if diff < self.epsilon {
                break;
            }
        }

        // Return as HashMap
        node_list
            .into_iter()
            .enumerate()
            .map(|(i, id)| (id, scores[i]))
            .collect()
    }

    /// Compute PPR and return only the top-k highest scoring nodes.
    pub fn compute_top_k(
        &self,
        adjacency: &HashMap<Uuid, Vec<(Uuid, f64)>>,
        seed_nodes: &HashMap<Uuid, f64>,
        k: usize,
    ) -> Vec<(Uuid, f64)> {
        let all_scores = self.compute(adjacency, seed_nodes);
        let mut ranked: Vec<(Uuid, f64)> = all_scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(k);
        ranked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ppr_seed_node_has_highest_score() {
        let mut adjacency = HashMap::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        adjacency.insert(a, vec![(b, 1.0)]);
        adjacency.insert(b, vec![(c, 1.0), (a, 1.0)]);
        adjacency.insert(c, vec![(a, 1.0)]);

        let mut seeds = HashMap::new();
        seeds.insert(a, 1.0);

        let ppr = PersonalizedPageRank::default_params();
        let scores = ppr.compute(&adjacency, &seeds);

        assert!(
            scores[&a] > scores[&b],
            "Seed node should have highest score"
        );
        assert!(
            scores[&a] > scores[&c],
            "Seed node should score above non-seeds"
        );
    }

    #[test]
    fn test_ppr_empty_graph() {
        let adjacency = HashMap::new();
        let seeds = HashMap::new();
        let ppr = PersonalizedPageRank::default_params();
        let scores = ppr.compute(&adjacency, &seeds);
        assert!(scores.is_empty());
    }

    #[test]
    fn test_ppr_star_topology() {
        let mut adjacency = HashMap::new();
        let center = Uuid::new_v4();
        let spokes: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();

        for spoke in &spokes {
            adjacency.insert(center, spokes.iter().map(|s| (*s, 1.0)).collect());
            adjacency.insert(*spoke, vec![(center, 1.0)]);
        }

        let mut seeds = HashMap::new();
        seeds.insert(spokes[0], 1.0);

        let ppr = PersonalizedPageRank::default_params();
        let scores = ppr.compute(&adjacency, &seeds);

        // Center should score high (hub)
        assert!(scores[&center] > 0.0);
        // Seed spoke should score highest among spokes
        let seed_score = scores[&spokes[0]];
        for spoke in &spokes[1..] {
            assert!(
                seed_score >= scores[spoke],
                "Seed spoke should score >= non-seed spokes"
            );
        }
    }

    #[test]
    fn test_ppr_top_k() {
        let mut adjacency = HashMap::new();
        let nodes: Vec<Uuid> = (0..10).map(|_| Uuid::new_v4()).collect();

        // Chain: 0->1->2->...->9
        for i in 0..9 {
            adjacency.insert(nodes[i], vec![(nodes[i + 1], 1.0)]);
        }
        adjacency.insert(nodes[9], vec![(nodes[0], 1.0)]);

        let mut seeds = HashMap::new();
        seeds.insert(nodes[0], 1.0);

        let ppr = PersonalizedPageRank::default_params();
        let top3 = ppr.compute_top_k(&adjacency, &seeds, 3);

        assert_eq!(top3.len(), 3);
        assert_eq!(top3[0].0, nodes[0], "Seed should be #1");
    }

    #[test]
    fn test_ppr_scores_sum_to_approximately_one() {
        let mut adjacency = HashMap::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        adjacency.insert(a, vec![(b, 1.0), (c, 1.0)]);
        adjacency.insert(b, vec![(a, 1.0)]);
        adjacency.insert(c, vec![(b, 1.0)]);

        let mut seeds = HashMap::new();
        seeds.insert(a, 1.0);

        let ppr = PersonalizedPageRank::default_params();
        let scores = ppr.compute(&adjacency, &seeds);
        let total: f64 = scores.values().sum();
        assert!(
            (total - 1.0).abs() < 0.01,
            "PPR scores should sum to ~1.0, got {}",
            total
        );
    }
}
