// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! DRIFT Search: Dynamic Reasoning and Inference with Flexible Traversal.
//!
//! DRIFT dynamically switches between local and global search strategies
//! based on query characteristics. For broad queries ("What are the main
//! research themes?"), it uses community summaries. For specific queries
//! ("Where does Alice work?"), it uses local entity traversal. For
//! ambiguous queries, it runs both and fuses results.
//!
//! Reference: "DRIFT Search is a GraphRAG query method that combines
//! global and local search strategies" (Microsoft GraphRAG, 2026).

use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

/// DRIFT Search: adaptive local/global retrieval.
pub struct DriftSearch {
    global_config: GlobalSearchConfig,
    local_config: LocalSearchConfig,
    /// Threshold for breadth classification (0.0–1.0).
    /// Above this → global, below (1-this) → local, otherwise → adaptive.
    adaptation_threshold: f64,
}

/// Configuration for the global (community-based) search path.
#[derive(Debug, Clone)]
pub struct GlobalSearchConfig {
    /// Number of community summaries to use.
    pub top_communities: usize,
    /// Maximum follow-up queries to generate.
    pub max_follow_ups: usize,
}

impl Default for GlobalSearchConfig {
    fn default() -> Self {
        Self {
            top_communities: 5,
            max_follow_ups: 5,
        }
    }
}

/// Configuration for the local (entity-traversal) search path.
#[derive(Debug, Clone)]
pub struct LocalSearchConfig {
    /// Maximum BFS hops from seed entities.
    pub max_hops: usize,
    /// Maximum entities to return.
    pub max_entities: usize,
}

impl Default for LocalSearchConfig {
    fn default() -> Self {
        Self {
            max_hops: 2,
            max_entities: 20,
        }
    }
}

/// Result from DRIFT search.
#[derive(Debug, Clone)]
pub struct DriftResult {
    /// Which strategy was selected.
    pub strategy_used: DriftStrategy,
    /// Node IDs found, scored and ranked.
    pub ranked_nodes: Vec<(Uuid, f64)>,
    /// Intermediate answers from each search phase.
    pub intermediate_answers: Vec<IntermediateAnswer>,
    /// Follow-up queries generated (for global path).
    pub follow_up_queries: Vec<String>,
    /// Total nodes visited across all phases.
    pub nodes_visited: usize,
    /// Estimated confidence (0.0–1.0).
    pub confidence: f64,
}

/// The strategy DRIFT chose for this query.
#[derive(Debug, Clone)]
pub enum DriftStrategy {
    /// Specific question — entity-local BFS.
    Local,
    /// Broad question — community summary map-reduce.
    Global,
    /// Ambiguous — ran both and fused.
    Adaptive {
        local_weight: f64,
        global_weight: f64,
    },
}

/// An intermediate answer from one phase of DRIFT.
#[derive(Debug, Clone)]
pub struct IntermediateAnswer {
    pub query: String,
    pub source: String,
    pub node_count: usize,
    pub confidence: f64,
}

/// A community summary for global search.
#[derive(Debug, Clone)]
pub struct CommunitySummary {
    pub community_id: Uuid,
    pub level: usize,
    pub summary: String,
    pub entity_count: usize,
    pub key_entities: Vec<String>,
    pub importance: f64,
}

impl DriftSearch {
    /// Create a DRIFT search with custom configuration.
    pub fn new(
        global_config: GlobalSearchConfig,
        local_config: LocalSearchConfig,
        adaptation_threshold: f64,
    ) -> Self {
        Self {
            global_config,
            local_config,
            adaptation_threshold,
        }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(
            GlobalSearchConfig::default(),
            LocalSearchConfig::default(),
            0.5,
        )
    }

    /// Execute DRIFT search.
    ///
    /// # Arguments
    /// * `query` — Natural language query
    /// * `community_summaries` — Pre-computed community summaries for global search
    /// * `entity_graph` — Adjacency: entity_id -> Vec<(neighbor_id, rel_type, weight)>
    /// * `entity_texts` — Entity texts for matching: entity_id -> text
    pub fn search(
        &self,
        query: &str,
        community_summaries: &[CommunitySummary],
        entity_graph: &HashMap<Uuid, Vec<(Uuid, String, f64)>>,
        entity_texts: &HashMap<Uuid, String>,
    ) -> DriftResult {
        let breadth = self.estimate_query_breadth(query);

        if breadth > self.adaptation_threshold {
            self.search_global(query, community_summaries, entity_graph, entity_texts)
        } else if breadth < (1.0 - self.adaptation_threshold) {
            self.search_local(query, entity_graph, entity_texts)
        } else {
            self.search_adaptive(query, community_summaries, entity_graph, entity_texts)
        }
    }

    /// Classify query breadth: 0.0 = very specific, 1.0 = very broad.
    fn estimate_query_breadth(&self, query: &str) -> f64 {
        let query_lower = query.to_lowercase();

        let broad_indicators = [
            "overview",
            "summary",
            "summarize",
            "all",
            "general",
            "broad",
            "landscape",
            "trend",
            "theme",
            "pattern",
            "main",
            "key",
            "compare",
            "overall",
            "what are the",
            "tell me about",
        ];
        let specific_indicators = [
            "who is",
            "where is",
            "where does",
            "when did",
            "how much",
            "what is the",
            "which",
            "specific",
            "exactly",
            "name",
        ];

        let broad_count = broad_indicators
            .iter()
            .filter(|kw| query_lower.contains(**kw))
            .count() as f64;
        let specific_count = specific_indicators
            .iter()
            .filter(|kw| query_lower.contains(**kw))
            .count() as f64;

        let word_count = query.split_whitespace().count() as f64;
        let length_factor = (word_count / 15.0).min(1.0);

        let keyword_factor = if broad_count + specific_count > 0.0 {
            broad_count / (broad_count + specific_count)
        } else {
            0.5 // Default to ambiguous
        };

        keyword_factor * 0.7 + length_factor * 0.3
    }

    /// Global search: rank community summaries, generate follow-ups, refine locally.
    fn search_global(
        &self,
        query: &str,
        community_summaries: &[CommunitySummary],
        entity_graph: &HashMap<Uuid, Vec<(Uuid, String, f64)>>,
        entity_texts: &HashMap<Uuid, String>,
    ) -> DriftResult {
        let mut intermediate_answers = Vec::new();
        let mut all_nodes: HashMap<Uuid, f64> = HashMap::new();

        // Step 1: Rank community summaries by relevance
        let ranked = self.rank_communities(query, community_summaries);

        // Step 2: Collect entities from top communities
        // (In production, resolve community member IDs. Here we use key entities.)
        for (cs, relevance) in ranked.iter().take(self.global_config.top_communities) {
            intermediate_answers.push(IntermediateAnswer {
                query: query.to_string(),
                source: format!("community_{}", cs.community_id),
                node_count: cs.entity_count,
                confidence: *relevance,
            });

            // Find entities matching community key entities
            for (entity_id, text) in entity_texts {
                let text_lower = text.to_lowercase();
                for key_entity in &cs.key_entities {
                    if text_lower.contains(&key_entity.to_lowercase()) {
                        let score = relevance * cs.importance;
                        all_nodes
                            .entry(*entity_id)
                            .and_modify(|s| *s = s.max(score))
                            .or_insert(score);
                    }
                }
            }
        }

        // Step 3: Generate follow-up queries and run local searches
        let follow_ups = self.generate_follow_ups(query, &ranked);
        for follow_up in &follow_ups {
            let local_nodes = self.local_bfs(follow_up, entity_graph, entity_texts);
            let count = local_nodes.len();
            for (id, score) in local_nodes {
                all_nodes
                    .entry(id)
                    .and_modify(|s| *s = s.max(score * 0.7))
                    .or_insert(score * 0.7);
            }
            if count > 0 {
                intermediate_answers.push(IntermediateAnswer {
                    query: follow_up.clone(),
                    source: "local_refinement".to_string(),
                    node_count: count,
                    confidence: 0.6,
                });
            }
        }

        let total_visited = all_nodes.len();
        let mut ranked_nodes: Vec<(Uuid, f64)> = all_nodes.into_iter().collect();
        ranked_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        DriftResult {
            strategy_used: DriftStrategy::Global,
            ranked_nodes,
            intermediate_answers,
            follow_up_queries: follow_ups,
            nodes_visited: total_visited,
            confidence: 0.65,
        }
    }

    /// Local search: BFS from query-matching seed entities.
    fn search_local(
        &self,
        query: &str,
        entity_graph: &HashMap<Uuid, Vec<(Uuid, String, f64)>>,
        entity_texts: &HashMap<Uuid, String>,
    ) -> DriftResult {
        let nodes = self.local_bfs(query, entity_graph, entity_texts);
        let total = nodes.len();

        let intermediate_answers = vec![IntermediateAnswer {
            query: query.to_string(),
            source: "local_bfs".to_string(),
            node_count: total,
            confidence: 0.8,
        }];

        DriftResult {
            strategy_used: DriftStrategy::Local,
            ranked_nodes: nodes,
            intermediate_answers,
            follow_up_queries: Vec::new(),
            nodes_visited: total,
            confidence: 0.8,
        }
    }

    /// Adaptive: run both strategies and merge.
    fn search_adaptive(
        &self,
        query: &str,
        community_summaries: &[CommunitySummary],
        entity_graph: &HashMap<Uuid, Vec<(Uuid, String, f64)>>,
        entity_texts: &HashMap<Uuid, String>,
    ) -> DriftResult {
        let global = self.search_global(query, community_summaries, entity_graph, entity_texts);
        let local = self.search_local(query, entity_graph, entity_texts);

        // Merge node scores with dynamic weighting
        let local_weight = if local.nodes_visited > 0 { 0.6 } else { 0.3 };
        let global_weight = 1.0 - local_weight;

        let mut merged: HashMap<Uuid, f64> = HashMap::new();
        for (id, score) in &global.ranked_nodes {
            *merged.entry(*id).or_default() += score * global_weight;
        }
        for (id, score) in &local.ranked_nodes {
            *merged.entry(*id).or_default() += score * local_weight;
        }

        let mut ranked_nodes: Vec<(Uuid, f64)> = merged.into_iter().collect();
        ranked_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut all_intermediates = global.intermediate_answers;
        all_intermediates.extend(local.intermediate_answers);

        DriftResult {
            strategy_used: DriftStrategy::Adaptive {
                local_weight,
                global_weight,
            },
            ranked_nodes,
            intermediate_answers: all_intermediates,
            follow_up_queries: global.follow_up_queries,
            nodes_visited: global.nodes_visited + local.nodes_visited,
            confidence: global.confidence * global_weight + local.confidence * local_weight,
        }
    }

    /// BFS from query-matching seed entities up to `max_hops`.
    fn local_bfs(
        &self,
        query: &str,
        entity_graph: &HashMap<Uuid, Vec<(Uuid, String, f64)>>,
        entity_texts: &HashMap<Uuid, String>,
    ) -> Vec<(Uuid, f64)> {
        let query_lower = query.to_lowercase();
        let query_words: HashSet<&str> = query_lower.split_whitespace().collect();

        // Find seed entities by text matching
        let mut seeds: Vec<(Uuid, f64)> = entity_texts
            .iter()
            .filter_map(|(id, text)| {
                let text_lower = text.to_lowercase();
                let text_words: HashSet<&str> = text_lower.split_whitespace().collect();
                let overlap = query_words.intersection(&text_words).count();
                if overlap > 0 {
                    let score = overlap as f64 / query_words.len().max(1) as f64;
                    Some((*id, score))
                } else {
                    None
                }
            })
            .collect();
        seeds.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // BFS expansion
        let mut visited: HashSet<Uuid> = HashSet::new();
        let mut results: HashMap<Uuid, f64> = HashMap::new();
        let mut queue: VecDeque<(Uuid, f64, usize)> = seeds
            .iter()
            .take(10) // Max 10 seeds
            .map(|(id, score)| (*id, *score, 0))
            .collect();

        while let Some((node_id, score, depth)) = queue.pop_front() {
            if visited.contains(&node_id) || depth > self.local_config.max_hops {
                continue;
            }
            if results.len() >= self.local_config.max_entities {
                break;
            }

            visited.insert(node_id);
            results
                .entry(node_id)
                .and_modify(|s| *s = s.max(score))
                .or_insert(score);

            // Expand neighbors
            if let Some(neighbors) = entity_graph.get(&node_id) {
                for (neighbor_id, _rel_type, weight) in neighbors {
                    if !visited.contains(neighbor_id) {
                        let decayed = score * weight * 0.7; // Decay per hop
                        queue.push_back((*neighbor_id, decayed, depth + 1));
                    }
                }
            }
        }

        let mut ranked: Vec<(Uuid, f64)> = results.into_iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }

    /// Rank community summaries by keyword overlap with query.
    fn rank_communities(
        &self,
        query: &str,
        summaries: &[CommunitySummary],
    ) -> Vec<(CommunitySummary, f64)> {
        let query_lower = query.to_lowercase();
        let query_words: HashSet<&str> = query_lower.split_whitespace().collect();

        let mut ranked: Vec<(CommunitySummary, f64)> = summaries
            .iter()
            .map(|cs| {
                let summary_lower = cs.summary.to_lowercase();
                let summary_words: HashSet<&str> = summary_lower.split_whitespace().collect();

                let overlap = query_words.intersection(&summary_words).count() as f64;
                let score = overlap / query_words.len().max(1) as f64;

                // Boost by community importance
                let adjusted = score * (0.5 + 0.5 * cs.importance);

                (cs.clone(), adjusted)
            })
            .collect();

        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }

    /// Generate follow-up queries from top community key entities.
    fn generate_follow_ups(
        &self,
        query: &str,
        ranked_communities: &[(CommunitySummary, f64)],
    ) -> Vec<String> {
        let mut follow_ups = Vec::new();

        for (cs, _) in ranked_communities.iter().take(3) {
            for entity in cs.key_entities.iter().take(2) {
                follow_ups.push(format!("{} {}", query, entity));
            }
        }

        follow_ups.truncate(self.global_config.max_follow_ups);
        follow_ups
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_graph() -> (
        HashMap<Uuid, Vec<(Uuid, String, f64)>>,
        HashMap<Uuid, String>,
    ) {
        let alice = Uuid::new_v4();
        let anthropic = Uuid::new_v4();
        let bob = Uuid::new_v4();
        let openai = Uuid::new_v4();

        let mut graph = HashMap::new();
        graph.insert(alice, vec![(anthropic, "works_at".to_string(), 1.0)]);
        graph.insert(bob, vec![(openai, "works_at".to_string(), 1.0)]);
        graph.insert(anthropic, vec![(alice, "employs".to_string(), 1.0)]);
        graph.insert(openai, vec![(bob, "employs".to_string(), 1.0)]);

        let mut texts = HashMap::new();
        texts.insert(alice, "Alice researcher".to_string());
        texts.insert(anthropic, "Anthropic AI safety company".to_string());
        texts.insert(bob, "Bob engineer".to_string());
        texts.insert(openai, "OpenAI large language models".to_string());

        (graph, texts)
    }

    #[test]
    fn test_specific_query_uses_local() {
        let drift = DriftSearch::with_defaults();
        let (graph, texts) = make_test_graph();
        let communities = vec![];

        let result = drift.search("Where does Alice work?", &communities, &graph, &texts);

        match result.strategy_used {
            DriftStrategy::Local | DriftStrategy::Adaptive { .. } => {}
            DriftStrategy::Global => panic!("Specific query should not use global strategy"),
        }
    }

    #[test]
    fn test_broad_query_uses_global() {
        let drift = DriftSearch::with_defaults();
        let (graph, texts) = make_test_graph();

        let communities = vec![CommunitySummary {
            community_id: Uuid::new_v4(),
            level: 0,
            summary: "AI research organizations including Anthropic and OpenAI".to_string(),
            entity_count: 4,
            key_entities: vec!["Anthropic".to_string(), "OpenAI".to_string()],
            importance: 0.9,
        }];

        let result = drift.search(
            "Give me an overview of all the main AI research themes and trends",
            &communities,
            &graph,
            &texts,
        );

        match result.strategy_used {
            DriftStrategy::Global | DriftStrategy::Adaptive { .. } => {}
            DriftStrategy::Local => panic!("Broad query should use global or adaptive"),
        }
    }

    #[test]
    fn test_local_bfs_finds_connected_entities() {
        let drift = DriftSearch::with_defaults();
        let (graph, texts) = make_test_graph();

        let results = drift.local_bfs("Alice", &graph, &texts);
        assert!(
            !results.is_empty(),
            "Should find Alice and connected entities"
        );
    }

    #[test]
    fn test_breadth_estimation() {
        let drift = DriftSearch::with_defaults();

        let specific = drift.estimate_query_breadth("Where does Alice work?");
        let broad = drift.estimate_query_breadth(
            "Give me a broad overview of all the main research themes and trends",
        );

        assert!(
            broad > specific,
            "Broad query ({}) should score higher than specific ({})",
            broad,
            specific
        );
    }
}
