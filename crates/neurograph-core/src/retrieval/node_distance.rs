// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Node distance reranker.
//!
//! Reranks search results by graph distance from a center (focal) node.
//! Closer nodes score higher. Uses BFS to compute shortest-path distances.
//!
//! Closes a competitive gap — Graphiti has `NodeReranker.node_distance`
//! and `EdgeReranker.node_distance` for graph-proximity boosting.

use std::collections::{HashMap, VecDeque};

/// Reranks results by graph distance from a center node.
///
/// Items closer to the center node receive higher scores.
/// Uses exponential decay: `score = decay^distance`.
///
/// # Example
///
/// ```rust
/// use neurograph_core::retrieval::node_distance::NodeDistanceReranker;
///
/// let reranker = NodeDistanceReranker::new(3, 0.5);
/// // Distance 0 (center): score = 1.0
/// // Distance 1: score = 0.5
/// // Distance 2: score = 0.25
/// // Distance 3: score = 0.125
/// // Distance 4+: score = 0.0 (unreachable within max_depth)
/// ```
#[derive(Debug, Clone)]
pub struct NodeDistanceReranker {
    /// Maximum BFS depth to explore.
    max_depth: usize,
    /// Decay factor per hop: score = decay^distance.
    /// 0.5 = each hop halves the score.
    decay: f32,
}

impl NodeDistanceReranker {
    /// Create a new node distance reranker.
    ///
    /// - `max_depth`: Maximum BFS depth (typically 2-5)
    /// - `decay`: Score decay per hop (0.0-1.0, typically 0.5-0.8)
    pub fn new(max_depth: usize, decay: f32) -> Self {
        Self {
            max_depth: max_depth.max(1),
            decay: decay.clamp(0.0, 1.0),
        }
    }

    /// Default configuration: max_depth=3, decay=0.5.
    pub fn default_config() -> Self {
        Self::new(3, 0.5)
    }

    /// Compute shortest distances from center_node to all reachable nodes via BFS.
    ///
    /// Takes a neighbor lookup function to remain storage-agnostic.
    /// Returns a map of `node_id → distance`.
    pub fn compute_distances<F>(
        &self,
        center_node_id: &str,
        get_neighbors: F,
    ) -> HashMap<String, usize>
    where
        F: Fn(&str) -> Vec<String>,
    {
        let mut distances: HashMap<String, usize> = HashMap::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();

        distances.insert(center_node_id.to_string(), 0);
        queue.push_back((center_node_id.to_string(), 0));

        while let Some((node_id, depth)) = queue.pop_front() {
            if depth >= self.max_depth {
                continue;
            }

            let neighbors = get_neighbors(&node_id);
            for neighbor_id in neighbors {
                if !distances.contains_key(&neighbor_id) {
                    distances.insert(neighbor_id.clone(), depth + 1);
                    queue.push_back((neighbor_id, depth + 1));
                }
            }
        }

        distances
    }

    /// Compute the distance-based score for a given distance.
    ///
    /// - Distance 0 (the center node) = 1.0
    /// - Beyond max_depth = 0.0
    pub fn score_for_distance(&self, distance: usize) -> f32 {
        if distance > self.max_depth {
            0.0
        } else {
            self.decay.powi(distance as i32)
        }
    }

    /// Rerank items by their graph distance to the center node.
    ///
    /// Items not reachable within `max_depth` get score 0.0.
    /// Items are returned sorted by score descending.
    pub fn rerank<T: Clone>(
        &self,
        items: &[(T, String)], // (item, node_id)
        distances: &HashMap<String, usize>,
    ) -> Vec<(T, f32)> {
        let mut scored: Vec<(T, f32)> = items
            .iter()
            .map(|(item, node_id)| {
                let distance = distances
                    .get(node_id)
                    .copied()
                    .unwrap_or(self.max_depth + 1);
                let score = self.score_for_distance(distance);
                (item.clone(), score)
            })
            .collect();

        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        scored
    }

    /// Combine distance score with an existing relevance score.
    ///
    /// Combined = α · relevance + (1-α) · distance_score
    pub fn combine_scores(
        &self,
        relevance_score: f32,
        distance: usize,
        alpha: f32,
    ) -> f32 {
        let distance_score = self.score_for_distance(distance);
        alpha * relevance_score + (1.0 - alpha) * distance_score
    }
}

impl Default for NodeDistanceReranker {
    fn default() -> Self {
        Self::default_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_graph() -> HashMap<String, Vec<String>> {
        // A -- B -- C -- D
        // |         |
        // E         F
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        graph.insert("A".into(), vec!["B".into(), "E".into()]);
        graph.insert("B".into(), vec!["A".into(), "C".into()]);
        graph.insert("C".into(), vec!["B".into(), "D".into(), "F".into()]);
        graph.insert("D".into(), vec!["C".into()]);
        graph.insert("E".into(), vec!["A".into()]);
        graph.insert("F".into(), vec!["C".into()]);
        graph
    }

    #[test]
    fn test_compute_distances() {
        let graph = build_test_graph();
        let reranker = NodeDistanceReranker::new(5, 0.5);

        let distances = reranker.compute_distances("A", |node_id| {
            graph.get(node_id).cloned().unwrap_or_default()
        });

        assert_eq!(distances["A"], 0);
        assert_eq!(distances["B"], 1);
        assert_eq!(distances["E"], 1);
        assert_eq!(distances["C"], 2);
        assert_eq!(distances["D"], 3);
        assert_eq!(distances["F"], 3);
    }

    #[test]
    fn test_compute_distances_max_depth() {
        let graph = build_test_graph();
        let reranker = NodeDistanceReranker::new(1, 0.5);

        let distances = reranker.compute_distances("A", |node_id| {
            graph.get(node_id).cloned().unwrap_or_default()
        });

        assert_eq!(distances["A"], 0);
        assert_eq!(distances["B"], 1);
        assert_eq!(distances["E"], 1);
        assert!(!distances.contains_key("C")); // Depth 2 - beyond max
    }

    #[test]
    fn test_score_for_distance() {
        let reranker = NodeDistanceReranker::new(3, 0.5);

        assert!((reranker.score_for_distance(0) - 1.0).abs() < f32::EPSILON);
        assert!((reranker.score_for_distance(1) - 0.5).abs() < f32::EPSILON);
        assert!((reranker.score_for_distance(2) - 0.25).abs() < f32::EPSILON);
        assert!((reranker.score_for_distance(3) - 0.125).abs() < f32::EPSILON);
        assert!((reranker.score_for_distance(4) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_rerank() {
        let graph = build_test_graph();
        let reranker = NodeDistanceReranker::new(5, 0.5);

        let distances = reranker.compute_distances("A", |node_id| {
            graph.get(node_id).cloned().unwrap_or_default()
        });

        let items = vec![
            ("D_item", "D".to_string()), // Distance 3
            ("A_item", "A".to_string()), // Distance 0
            ("C_item", "C".to_string()), // Distance 2
        ];

        let ranked = reranker.rerank(&items, &distances);
        assert_eq!(ranked[0].0, "A_item"); // Closest
        assert_eq!(ranked[1].0, "C_item");
        assert_eq!(ranked[2].0, "D_item"); // Farthest
    }

    #[test]
    fn test_combine_scores() {
        let reranker = NodeDistanceReranker::new(3, 0.5);

        // α = 0.7: 70% relevance, 30% distance
        let combined = reranker.combine_scores(0.9, 1, 0.7);
        // = 0.7 * 0.9 + 0.3 * 0.5 = 0.63 + 0.15 = 0.78
        assert!((combined - 0.78).abs() < 0.01);
    }
}
