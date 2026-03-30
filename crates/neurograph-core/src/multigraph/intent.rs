// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Intent-aware query router for multi-graph memory.
//!
//! Classifies query intent and routes to the appropriate graph views.
//! A factual query goes to Semantic+Entity, a temporal query goes to
//! Temporal+Semantic, a causal query goes to Causal+Entity, etc.

use super::GraphView;
use serde::{Deserialize, Serialize};

/// Classified query intent.
#[derive(Debug, Clone)]
pub struct QueryIntent {
    pub primary: IntentType,
    pub secondary: Option<IntentType>,
    pub confidence: f64,
    pub explanation: String,
}

/// Types of query intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntentType {
    /// "Who is Alice?" — entity lookup
    Factual,
    /// "What happened in 2024?" — time-scoped
    Temporal,
    /// "Why did X happen?" — cause-effect
    Causal,
    /// "What if X hadn't happened?" — counterfactual
    Hypothetical,
    /// "Compare X and Y" — multi-entity
    Comparative,
    /// "How many people work at X?" — counting/aggregation
    Aggregative,
    /// "Who does Alice's boss report to?" — chain traversal
    MultiHop,
    /// "Tell me about X" — open-ended
    Exploratory,
}

/// A traversal plan generated from classified intent.
/// Controls how deep and wide the multi-graph query traverses.
#[derive(Debug, Clone)]
pub struct TraversalPlan {
    /// Which graph views to query, with weights.
    pub views: Vec<(GraphView, f64)>,
    /// Maximum traversal depth (hops from seed).
    pub max_hops: usize,
    /// Maximum nodes to visit per graph view.
    pub max_nodes_per_graph: usize,
    /// Fusion strategy to use.
    pub fusion_strategy: FusionStrategy,
}

/// How to fuse results from multiple graph views.
#[derive(Debug, Clone, Copy)]
pub enum FusionStrategy {
    /// Reciprocal Rank Fusion with given k parameter.
    ReciprocalRankFusion { k: usize },
    /// Weighted sum of normalized scores.
    WeightedSum,
    /// Take the maximum score across views.
    MaxScore,
}

/// Routes queries to the appropriate graph views based on intent.
pub struct IntentRouter {
    /// View selection rules per intent type.
    view_weights: Vec<(IntentType, Vec<(GraphView, f64)>)>,
}

impl IntentRouter {
    pub fn new() -> Self {
        Self {
            view_weights: vec![
                (
                    IntentType::Factual,
                    vec![
                        (GraphView::Entity, 0.5),
                        (GraphView::Semantic, 0.4),
                        (GraphView::Temporal, 0.1),
                    ],
                ),
                (
                    IntentType::Temporal,
                    vec![
                        (GraphView::Temporal, 0.6),
                        (GraphView::Semantic, 0.2),
                        (GraphView::Entity, 0.2),
                    ],
                ),
                (
                    IntentType::Causal,
                    vec![
                        (GraphView::Causal, 0.5),
                        (GraphView::Entity, 0.3),
                        (GraphView::Temporal, 0.2),
                    ],
                ),
                (
                    IntentType::Hypothetical,
                    vec![
                        (GraphView::Causal, 0.4),
                        (GraphView::Entity, 0.3),
                        (GraphView::Semantic, 0.2),
                        (GraphView::Temporal, 0.1),
                    ],
                ),
                (
                    IntentType::Comparative,
                    vec![
                        (GraphView::Entity, 0.4),
                        (GraphView::Semantic, 0.4),
                        (GraphView::Temporal, 0.2),
                    ],
                ),
                (
                    IntentType::Aggregative,
                    vec![
                        (GraphView::Entity, 0.5),
                        (GraphView::Semantic, 0.3),
                        (GraphView::Temporal, 0.2),
                    ],
                ),
                (
                    IntentType::MultiHop,
                    vec![
                        (GraphView::Entity, 0.4),
                        (GraphView::Causal, 0.35),
                        (GraphView::Semantic, 0.25),
                    ],
                ),
                (
                    IntentType::Exploratory,
                    vec![
                        (GraphView::Semantic, 0.3),
                        (GraphView::Entity, 0.3),
                        (GraphView::Temporal, 0.2),
                        (GraphView::Causal, 0.2),
                    ],
                ),
            ],
        }
    }

    /// Classify query intent using keyword matching heuristics.
    pub fn classify(&self, query: &str) -> QueryIntent {
        let query_lower = query.to_lowercase();

        // Score each intent type
        let mut scores: Vec<(IntentType, f64, &str)> = vec![
            (IntentType::Factual, 0.0, "Default"),
            (IntentType::Temporal, 0.0, ""),
            (IntentType::Causal, 0.0, ""),
            (IntentType::Hypothetical, 0.0, ""),
            (IntentType::Comparative, 0.0, ""),
            (IntentType::Aggregative, 0.0, ""),
            (IntentType::MultiHop, 0.0, ""),
            (IntentType::Exploratory, 0.0, ""),
        ];

        // Temporal signals
        let temporal_keywords = [
            "when", "what year", "what month", "timeline", "history",
            "before", "after", "during", "since", "until", "between",
            "date", "time", "recent", "latest", "earliest", "first", "last",
            "in 2", "in 1", "happened",
        ];
        for kw in &temporal_keywords {
            if query_lower.contains(kw) {
                scores[1].1 += 1.0;
            }
        }

        // Causal signals
        let causal_keywords = [
            "why", "cause", "because", "reason", "led to",
            "result", "effect", "consequence", "impact", "due to",
            "how did", "what caused", "root cause",
        ];
        for kw in &causal_keywords {
            if query_lower.contains(kw) {
                scores[2].1 += 1.0;
            }
        }

        // Hypothetical signals
        let hypo_keywords = [
            "what if", "what would", "hypothetically", "had not",
            "hadn't", "without", "suppose", "imagine", "could have",
            "counterfactual",
        ];
        for kw in &hypo_keywords {
            if query_lower.contains(kw) {
                scores[3].1 += 1.5; // Stronger signal
            }
        }

        // Comparative signals
        let compare_keywords = [
            "compare", "versus", "vs", "difference", "differ",
            "similar", "contrast", "better", "worse", "advantage",
            "between", "compared to",
        ];
        for kw in &compare_keywords {
            if query_lower.contains(kw) {
                scores[4].1 += 1.0;
            }
        }

        // Aggregative signals
        let agg_keywords = [
            "how many", "how much", "count", "total", "number of",
            "average", "most", "least", "percentage", "all",
            "list all", "every",
        ];
        for kw in &agg_keywords {
            if query_lower.contains(kw) {
                scores[5].1 += 1.0;
            }
        }

        // Exploratory signals
        let explore_keywords = [
            "tell me about", "explain", "describe", "overview",
            "what is", "who is", "summarize", "introduction",
        ];
        for kw in &explore_keywords {
            if query_lower.contains(kw) {
                scores[6].1 += 0.8;
            }
        }

        // Multi-hop signals
        let multihop_keywords = [
            "who does", "through", "chain", "connected to",
            "relationship between", "path from", "report to",
            "boss of", "manager of", "leads to", "via",
        ];
        for kw in &multihop_keywords {
            if query_lower.contains(kw) {
                scores[6].1 += 1.2;
            }
        }

        // Factual signals (base, slightly boosted)
        let factual_keywords = [
            "who", "what", "where", "which", "name",
            "is", "are", "does", "do",
        ];
        for kw in &factual_keywords {
            if query_lower.starts_with(kw) || query_lower.contains(&format!(" {} ", kw)) {
                scores[0].1 += 0.5;
            }
        }

        // If no strong signals, default to exploratory
        let max_score = scores.iter().map(|(_, s, _)| *s).fold(0.0f64, f64::max);
        if max_score < 0.5 {
            scores[7].1 += 1.0; // Exploratory default
        }

        // Sort by score to find primary and secondary
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let primary = scores[0].0;
        let secondary = if scores.len() > 1 && scores[1].1 > 0.5 {
            Some(scores[1].0)
        } else {
            None
        };

        let confidence = if max_score > 0.0 {
            (scores[0].1 / (scores[0].1 + scores.get(1).map(|s| s.1).unwrap_or(0.0))).min(1.0)
        } else {
            0.5
        };

        QueryIntent {
            primary,
            secondary,
            confidence,
            explanation: format!("{:?} (score: {:.1})", primary, scores[0].1),
        }
    }

    /// Select which graph views to query based on classified intent.
    pub fn select_views(&self, intent: &QueryIntent) -> Vec<GraphView> {
        let mut views = Vec::new();

        // Primary intent views
        if let Some((_, weights)) = self.view_weights.iter().find(|(t, _)| *t == intent.primary) {
            for (view, weight) in weights {
                if *weight >= 0.1 {
                    views.push(*view);
                }
            }
        }

        // Add secondary intent views if present
        if let Some(secondary) = &intent.secondary {
            if let Some((_, weights)) = self.view_weights.iter().find(|(t, _)| t == secondary) {
                for (view, _) in weights {
                    if !views.contains(view) {
                        views.push(*view);
                    }
                }
            }
        }

        // Ensure at least semantic view is included
        if !views.contains(&GraphView::Semantic) {
            views.push(GraphView::Semantic);
        }

        views
    }

    /// Get the weight for a specific view under a specific intent.
    pub fn view_weight(&self, intent_type: IntentType, view: GraphView) -> f64 {
        self.view_weights
            .iter()
            .find(|(t, _)| *t == intent_type)
            .and_then(|(_, weights)| weights.iter().find(|(v, _)| *v == view))
            .map(|(_, w)| *w)
            .unwrap_or(0.0)
    }

    /// Generate a full traversal plan from classified intent.
    pub fn plan(&self, intent: &QueryIntent) -> TraversalPlan {
        let views: Vec<(GraphView, f64)> = self
            .view_weights
            .iter()
            .find(|(t, _)| *t == intent.primary)
            .map(|(_, w)| w.clone())
            .unwrap_or_else(|| vec![(GraphView::Semantic, 1.0)]);

        let (max_hops, max_nodes_per_graph, fusion_strategy) = match intent.primary {
            IntentType::Factual => (2, 20, FusionStrategy::ReciprocalRankFusion { k: 60 }),
            IntentType::Temporal => (3, 30, FusionStrategy::ReciprocalRankFusion { k: 60 }),
            IntentType::Causal => (4, 25, FusionStrategy::WeightedSum),
            IntentType::Hypothetical => (3, 25, FusionStrategy::WeightedSum),
            IntentType::Comparative => (3, 20, FusionStrategy::ReciprocalRankFusion { k: 60 }),
            IntentType::Aggregative => (2, 40, FusionStrategy::ReciprocalRankFusion { k: 60 }),
            IntentType::MultiHop => (5, 40, FusionStrategy::WeightedSum),
            IntentType::Exploratory => (3, 30, FusionStrategy::ReciprocalRankFusion { k: 60 }),
        };

        TraversalPlan {
            views,
            max_hops,
            max_nodes_per_graph,
            fusion_strategy,
        }
    }
}

impl Default for IntentRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_temporal() {
        let router = IntentRouter::new();
        let intent = router.classify("When did Alice join Anthropic?");
        assert_eq!(intent.primary, IntentType::Temporal);
    }

    #[test]
    fn test_classify_causal() {
        let router = IntentRouter::new();
        let intent = router.classify("Why did the project fail? What caused it?");
        assert_eq!(intent.primary, IntentType::Causal);
    }

    #[test]
    fn test_classify_hypothetical() {
        let router = IntentRouter::new();
        let intent = router.classify("What if Alice had not joined Anthropic?");
        assert_eq!(intent.primary, IntentType::Hypothetical);
    }

    #[test]
    fn test_classify_comparative() {
        let router = IntentRouter::new();
        let intent = router.classify("Compare Anthropic versus OpenAI");
        assert_eq!(intent.primary, IntentType::Comparative);
    }

    #[test]
    fn test_view_selection_always_includes_semantic() {
        let router = IntentRouter::new();
        let intent = router.classify("Why did this happen?");
        let views = router.select_views(&intent);
        assert!(
            views.contains(&GraphView::Semantic),
            "Semantic should always be included"
        );
    }

    #[test]
    fn test_causal_intent_includes_causal_view() {
        let router = IntentRouter::new();
        let intent = router.classify("What caused the crash?");
        let views = router.select_views(&intent);
        assert!(views.contains(&GraphView::Causal));
    }
}
