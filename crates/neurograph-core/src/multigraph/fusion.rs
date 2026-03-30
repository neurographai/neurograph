// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Cross-view fusion engine for multi-graph query results.
//!
//! Merges subgraph results from multiple graph views using
//! type-aligned Reciprocal Rank Fusion (RRF) with intent-based
//! weighting and cross-view reinforcement boosting.

use super::*;
use std::collections::HashMap;

/// Fuses subgraph results from multiple graph views.
pub struct FusionEngine {
    /// RRF constant (higher = more weight to lower-ranked items).
    rrf_k: f64,
    /// Cross-view reinforcement boost (applied when a node appears in multiple views).
    reinforcement_factor: f64,
}

impl FusionEngine {
    pub fn new() -> Self {
        Self {
            rrf_k: 60.0,
            reinforcement_factor: 1.3,
        }
    }

    /// Create with custom parameters.
    pub fn with_params(rrf_k: f64, reinforcement_factor: f64) -> Self {
        Self {
            rrf_k,
            reinforcement_factor,
        }
    }

    /// Fuse subgraph results from multiple views into a single ranked list.
    pub fn fuse(
        &self,
        subgraph_results: Vec<SubgraphResult>,
        intent: &QueryIntent,
    ) -> QueryResult {
        let mut fused_scores: HashMap<Uuid, f64> = HashMap::new();
        let mut view_counts: HashMap<Uuid, usize> = HashMap::new();
        let mut views_used = Vec::new();
        let mut reasoning_trace = Vec::new();

        let intent_router = IntentRouter::new();

        for result in &subgraph_results {
            views_used.push(result.view);

            // Get the view weight based on intent
            let view_weight = intent_router.view_weight(intent.primary, result.view);

            reasoning_trace.push(ReasoningStep {
                view: result.view,
                operation: "RRF fusion".to_string(),
                nodes_visited: result.node_ids.len(),
                duration_ms: result.duration_ms,
                explanation: format!(
                    "{:?} view: {} nodes, weight={:.2}",
                    result.view,
                    result.node_ids.len(),
                    view_weight
                ),
            });

            // Apply RRF: score(d) = Σ view_weight / (rrf_k + rank(d))
            for (rank, (node_id, _original_score)) in result
                .node_ids
                .iter()
                .zip(result.scores.iter())
                .enumerate()
            {
                let rrf_score = view_weight / (self.rrf_k + rank as f64 + 1.0);
                *fused_scores.entry(*node_id).or_default() += rrf_score;
                *view_counts.entry(*node_id).or_default() += 1;
            }
        }

        // Cross-view reinforcement: boost nodes appearing in multiple views
        for (node_id, score) in fused_scores.iter_mut() {
            let count = view_counts.get(node_id).copied().unwrap_or(1);
            if count > 1 {
                let boost = self.reinforcement_factor.powi(count as i32 - 1);
                *score *= boost;
            }
        }

        // Sort by fused score
        let mut items: Vec<(Uuid, f64)> = fused_scores.into_iter().collect();
        items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        reasoning_trace.push(ReasoningStep {
            view: GraphView::Semantic, // placeholder
            operation: "final_ranking".to_string(),
            nodes_visited: items.len(),
            duration_ms: 0,
            explanation: format!(
                "Fused {} nodes from {} views, reinforced {} cross-view nodes",
                items.len(),
                views_used.len(),
                view_counts.values().filter(|&&c| c > 1).count()
            ),
        });

        QueryResult {
            items,
            views_used,
            reasoning_trace,
        }
    }
}

impl Default for FusionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fusion_basic() {
        let engine = FusionEngine::new();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();

        let results = vec![
            SubgraphResult {
                view: GraphView::Semantic,
                node_ids: vec![node_a, node_b],
                scores: vec![0.9, 0.7],
                edges: Vec::new(),
                duration_ms: 5,
            },
            SubgraphResult {
                view: GraphView::Entity,
                node_ids: vec![node_a],
                scores: vec![0.8],
                edges: Vec::new(),
                duration_ms: 3,
            },
        ];

        let intent = QueryIntent {
            primary: IntentType::Factual,
            secondary: None,
            confidence: 0.8,
            explanation: "test".to_string(),
        };

        let fused = engine.fuse(results, &intent);

        assert!(!fused.items.is_empty());
        // node_a should be top (appears in both views, gets cross-view boost)
        assert_eq!(fused.items[0].0, node_a);
        assert!(
            fused.items[0].1 > fused.items[1].1,
            "Cross-view node should score higher"
        );
    }

    #[test]
    fn test_cross_view_reinforcement() {
        let engine = FusionEngine::new();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();

        // node_a appears in 3 views, node_b in 1
        let results = vec![
            SubgraphResult {
                view: GraphView::Semantic,
                node_ids: vec![node_a, node_b],
                scores: vec![0.5, 1.0],
                edges: Vec::new(),
                duration_ms: 0,
            },
            SubgraphResult {
                view: GraphView::Temporal,
                node_ids: vec![node_a],
                scores: vec![0.5],
                edges: Vec::new(),
                duration_ms: 0,
            },
            SubgraphResult {
                view: GraphView::Causal,
                node_ids: vec![node_a],
                scores: vec![0.5],
                edges: Vec::new(),
                duration_ms: 0,
            },
        ];

        let intent = QueryIntent {
            primary: IntentType::Exploratory,
            secondary: None,
            confidence: 0.5,
            explanation: "test".to_string(),
        };

        let fused = engine.fuse(results, &intent);
        // node_a should win despite starting with lower individual scores
        assert_eq!(fused.items[0].0, node_a);
    }
}
