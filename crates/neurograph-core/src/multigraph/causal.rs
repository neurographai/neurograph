// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Causal graph view: directed edges representing cause-effect relationships.
//!
//! Extracts causal patterns using regex (offline) or LLM (online).
//! Supports forward chaining (effects) and backward chaining (root causes).

use super::*;
use std::collections::{HashMap, HashSet, VecDeque};

/// Causal graph: directed edges from causes to effects.
pub struct CausalGraph {
    /// Forward edges: cause_id -> Vec<(effect_id, confidence, mechanism)>
    forward_edges: HashMap<Uuid, Vec<CausalEdge>>,
    /// Reverse index: effect_id -> Vec<cause_id>
    reverse_edges: HashMap<Uuid, Vec<Uuid>>,
    /// Total edge count
    total_edges: usize,
    config: MultiGraphConfig,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CausalEdge {
    effect_id: Uuid,
    confidence: f64,
    evidence: Vec<Uuid>,
    mechanism: String,
}

/// Regex-based causal relation extraction for offline mode.
struct CausalExtractor {
    patterns: Vec<(regex::Regex, String, f64)>,
}

impl CausalExtractor {
    fn new() -> Self {
        let patterns = vec![
            (
                regex::Regex::new(r"(?i)(.+?)\s+(?:caused|led to|resulted in|triggered)\s+(.+)")
                    .unwrap(),
                "caused".to_string(),
                0.8,
            ),
            (
                regex::Regex::new(r"(?i)(?:because|since|due to)\s+(.+?),\s*(.+)").unwrap(),
                "because_of".to_string(),
                0.7,
            ),
            (
                regex::Regex::new(r"(?i)(.+?)\s+(?:therefore|thus|consequently|hence)\s+(.+)")
                    .unwrap(),
                "therefore".to_string(),
                0.75,
            ),
            (
                regex::Regex::new(r"(?i)if\s+(.+?)\s+then\s+(.+)").unwrap(),
                "conditional".to_string(),
                0.6,
            ),
            (
                regex::Regex::new(r"(?i)(.+?)\s+(?:enabled|allowed|made possible)\s+(.+)").unwrap(),
                "enabled".to_string(),
                0.7,
            ),
        ];
        Self { patterns }
    }

    /// Extract causal relations from text using regex patterns.
    fn extract(&self, text: &str) -> Vec<(String, String, String, f64)> {
        let mut relations = Vec::new();
        for (pattern, rel_type, confidence) in &self.patterns {
            if let Some(captures) = pattern.captures(text) {
                if let (Some(cause), Some(effect)) = (captures.get(1), captures.get(2)) {
                    relations.push((
                        cause.as_str().trim().to_string(),
                        effect.as_str().trim().to_string(),
                        rel_type.clone(),
                        *confidence,
                    ));
                }
            }
        }
        relations
    }
}

impl CausalGraph {
    pub fn new(config: &MultiGraphConfig) -> Self {
        Self {
            forward_edges: HashMap::new(),
            reverse_edges: HashMap::new(),
            total_edges: 0,
            config: config.clone(),
        }
    }

    /// Add a memory item and extract causal edges from its content.
    pub fn add_item(&mut self, item: &MemoryItem) -> Vec<GraphEdge> {
        let extractor = CausalExtractor::new();
        let relations = extractor.extract(&item.content);
        let mut new_edges = Vec::new();

        for (cause_text, effect_text, relation, confidence) in relations {
            if confidence < self.config.causal_confidence_threshold {
                continue;
            }

            // Use the item's entity_ids if available, otherwise create
            // placeholder causal links using the item ID itself
            if item.entity_ids.len() >= 2 {
                let cause_id = item.entity_ids[0];
                let effect_id = item.entity_ids[1];

                self.forward_edges
                    .entry(cause_id)
                    .or_default()
                    .push(CausalEdge {
                        effect_id,
                        confidence,
                        evidence: vec![item.id],
                        mechanism: format!("{}: {} -> {}", relation, cause_text, effect_text),
                    });

                self.reverse_edges
                    .entry(effect_id)
                    .or_default()
                    .push(cause_id);

                self.total_edges += 1;

                new_edges.push(GraphEdge {
                    id: Uuid::new_v4(),
                    source: cause_id,
                    target: effect_id,
                    view: GraphView::Causal,
                    relation,
                    weight: confidence,
                    valid_from: item.valid_from,
                    valid_until: item.valid_until,
                    metadata: {
                        let mut m = HashMap::new();
                        m.insert(
                            "mechanism".to_string(),
                            serde_json::json!(format!("{} -> {}", cause_text, effect_text)),
                        );
                        m
                    },
                });
            }
        }

        new_edges
    }

    /// Traverse causal chains forward (effects) and backward (causes).
    pub fn traverse(&self, _query: &str, opts: &QueryOptions) -> SubgraphResult {
        let start = std::time::Instant::now();
        let mut visited: HashSet<Uuid> = HashSet::new();
        let mut results = Vec::new();
        let max_depth = 5;

        // Seed with all known cause nodes
        let mut queue: VecDeque<(Uuid, f64, usize)> = VecDeque::new();
        for (cause_id, edges) in &self.forward_edges {
            let max_confidence = edges.iter().map(|e| e.confidence).fold(0.0f64, f64::max);
            queue.push_back((*cause_id, max_confidence, 0));
        }

        while let Some((node_id, score, depth)) = queue.pop_front() {
            if visited.contains(&node_id) || depth > max_depth {
                continue;
            }
            if results.len() >= opts.max_results {
                break;
            }

            visited.insert(node_id);
            results.push((node_id, score));

            // Forward: follow effects
            if let Some(effects) = self.forward_edges.get(&node_id) {
                for edge in effects {
                    if !visited.contains(&edge.effect_id) {
                        queue.push_back((edge.effect_id, score * edge.confidence, depth + 1));
                    }
                }
            }

            // Backward: follow causes
            if let Some(causes) = self.reverse_edges.get(&node_id) {
                for cause_id in causes {
                    if !visited.contains(cause_id) {
                        queue.push_back((*cause_id, score * 0.8, depth + 1));
                    }
                }
            }
        }

        SubgraphResult {
            view: GraphView::Causal,
            node_ids: results.iter().map(|(id, _)| *id).collect(),
            scores: results.iter().map(|(_, s)| *s).collect(),
            edges: Vec::new(),
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// Find all root causes for a given effect via backward chaining.
    pub fn root_causes(&self, effect_id: Uuid, max_depth: usize) -> Vec<Vec<Uuid>> {
        let mut paths = Vec::new();
        let mut stack = vec![(effect_id, vec![effect_id])];

        while let Some((current, path)) = stack.pop() {
            if path.len() > max_depth {
                continue;
            }
            match self.reverse_edges.get(&current) {
                Some(causes) if !causes.is_empty() => {
                    for cause_id in causes {
                        if !path.contains(cause_id) {
                            let mut new_path = path.clone();
                            new_path.push(*cause_id);
                            stack.push((*cause_id, new_path));
                        }
                    }
                }
                _ => {
                    paths.push(path);
                }
            }
        }

        paths
    }

    /// Number of causal edges.
    pub fn edge_count(&self) -> usize {
        self.total_edges
    }
}
