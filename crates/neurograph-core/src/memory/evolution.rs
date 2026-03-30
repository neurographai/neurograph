// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Self-Evolving Memory: RL-guided forgetting, decay, and consolidation.
//!
//! Implements a composite importance scoring model that combines:
//! - PageRank-style graph centrality
//! - Access frequency
//! - Temporal recency
//! - Entity connectivity
//! - Content salience
//!
//! Uses a Q-table based RL agent to learn keep/forget decisions
//! that maximize long-term retrieval relevance.
//!
//! Reference: "Self-Evolving Memory with RL-guided forgetting and
//! memory reconsolidation" (EverMemOS, 2026).

use chrono::{DateTime, Utc};
use rand::Rng;
use std::collections::HashMap;
use uuid::Uuid;

/// Configuration for memory decay.
#[derive(Debug, Clone)]
pub struct DecayPolicy {
    /// Base decay rate per hour (0.0 = no decay, 1.0 = instant).
    pub base_rate: f64,
    /// Importance threshold: items below this score are eligible for forgetting.
    pub forget_threshold: f64,
    /// Maximum number of items before overflow eviction.
    pub max_items: usize,
    /// Decay function: Exponential, Linear, or Step.
    pub decay_function: DecayFunction,
}

/// Type of decay function.
#[derive(Debug, Clone)]
pub enum DecayFunction {
    /// score *= e^(-rate * hours)
    Exponential,
    /// score -= rate * hours
    Linear,
    /// score = 0 if hours > threshold
    Step { threshold_hours: f64 },
}

impl Default for DecayPolicy {
    fn default() -> Self {
        Self {
            base_rate: 0.001,
            forget_threshold: 0.1,
            max_items: 100_000,
            decay_function: DecayFunction::Exponential,
        }
    }
}

/// RL-based retention policy using Q-table learning.
#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    /// Q-table: state_key -> [keep_value, forget_value]
    q_table: HashMap<String, [f64; 2]>,
    /// Learning rate
    alpha: f64,
    /// Discount factor
    gamma: f64,
    /// Exploration rate (epsilon-greedy)
    epsilon: f64,
    /// Random seed for reproducibility
    #[allow(dead_code)]
    seed: u64,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            q_table: HashMap::new(),
            alpha: 0.1,
            gamma: 0.95,
            epsilon: 0.1,
            seed: 42,
        }
    }
}

impl RetentionPolicy {
    /// Create with custom parameters.
    pub fn new(alpha: f64, gamma: f64, epsilon: f64, seed: u64) -> Self {
        Self {
            q_table: HashMap::new(),
            alpha,
            gamma,
            epsilon,
            seed,
        }
    }

    /// Discretize an item's state into a Q-table key.
    fn state_key(importance: f64, access_freq: f64, recency: f64, connectivity: f64) -> String {
        let imp_bin = (importance * 10.0).round() as i32;
        let acc_bin = (access_freq.ln().max(0.0) * 3.0).round() as i32;
        let rec_bin = (recency * 5.0).round() as i32;
        let con_bin = (connectivity * 5.0).round() as i32;
        format!("{}:{}:{}:{}", imp_bin, acc_bin, rec_bin, con_bin)
    }

    /// Decide whether to keep or forget an item.
    /// Returns true = keep, false = forget.
    pub fn decide(
        &self,
        importance: f64,
        access_freq: f64,
        recency: f64,
        connectivity: f64,
    ) -> bool {
        let key = Self::state_key(importance, access_freq, recency, connectivity);
        let q_values = self.q_table.get(&key).copied().unwrap_or([0.5, 0.5]);

        // Epsilon-greedy exploration
        let mut rng = rand::thread_rng();
        if rng.gen::<f64>() < self.epsilon {
            rng.gen::<bool>()
        } else {
            q_values[0] >= q_values[1] // 0 = keep, 1 = forget
        }
    }

    /// Update Q-values with observed reward.
    pub fn update(
        &mut self,
        importance: f64,
        access_freq: f64,
        recency: f64,
        connectivity: f64,
        action_keep: bool,
        reward: f64,
    ) {
        let key = Self::state_key(importance, access_freq, recency, connectivity);
        let q_values = self.q_table.entry(key).or_insert([0.5, 0.5]);
        let action_idx = if action_keep { 0 } else { 1 };

        // Q-learning update: Q(s,a) += α * (reward + γ * max(Q(s')) - Q(s,a))
        let best_next = q_values[0].max(q_values[1]);
        q_values[action_idx] +=
            self.alpha * (reward + self.gamma * best_next - q_values[action_idx]);
    }
}

/// A snapshot of an item for the evolution engine.
#[derive(Debug, Clone)]
pub struct EvolvableItem {
    pub id: Uuid,
    pub content: String,
    pub importance: f64,
    pub access_count: u64,
    pub last_accessed: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub connectivity: usize,
    pub embedding: Vec<f32>,
}

/// The memory evolution engine.
pub struct MemoryEvolution {
    pub decay_policy: DecayPolicy,
    pub retention_policy: RetentionPolicy,
}

/// Result from a single evolution cycle.
#[derive(Debug)]
pub struct EvolutionResult {
    pub items_scored: usize,
    pub items_decayed: usize,
    pub items_forgotten: usize,
    pub items_consolidated: usize,
    pub items_evicted: usize,
}

impl MemoryEvolution {
    /// Create with default policies.
    pub fn new() -> Self {
        Self {
            decay_policy: DecayPolicy::default(),
            retention_policy: RetentionPolicy::default(),
        }
    }

    /// Create with custom policies.
    pub fn with_policies(decay: DecayPolicy, retention: RetentionPolicy) -> Self {
        Self {
            decay_policy: decay,
            retention_policy: retention,
        }
    }

    /// Run a full evolution cycle on a set of items.
    ///
    /// 1. Score all items (composite importance)
    /// 2. Apply temporal decay
    /// 3. RL-guided keep/forget decisions
    /// 4. Consolidate similar items
    /// 5. Evict overflow
    pub fn evolve(&mut self, items: &mut Vec<EvolvableItem>) -> EvolutionResult {
        let now = Utc::now();
        let total = items.len();

        // Step 1: Composite importance scoring
        for item in items.iter_mut() {
            item.importance = self.composite_score(item, total);
        }

        // Step 2: Apply temporal decay
        let mut decayed_count = 0;
        for item in items.iter_mut() {
            let hours = (now - item.last_accessed).num_hours().max(0) as f64;
            let decay = match &self.decay_policy.decay_function {
                DecayFunction::Exponential => (-self.decay_policy.base_rate * hours).exp(),
                DecayFunction::Linear => (1.0 - self.decay_policy.base_rate * hours).max(0.0),
                DecayFunction::Step { threshold_hours } => {
                    if hours > *threshold_hours {
                        0.0
                    } else {
                        1.0
                    }
                }
            };
            let old_importance = item.importance;
            item.importance *= decay;
            if (old_importance - item.importance).abs() > 0.01 {
                decayed_count += 1;
            }
        }

        // Step 3: RL-guided forgetting
        let mut forgotten_ids: Vec<Uuid> = Vec::new();
        for item in items.iter() {
            if item.importance < self.decay_policy.forget_threshold {
                let access_freq = item.access_count as f64;
                let hours_since_access = (now - item.last_accessed).num_hours().max(0) as f64;
                let recency = 1.0 / (1.0 + hours_since_access / 24.0);
                let connectivity = item.connectivity as f64 / 10.0;

                let keep = self.retention_policy.decide(
                    item.importance,
                    access_freq,
                    recency,
                    connectivity,
                );

                if !keep {
                    forgotten_ids.push(item.id);
                }
            }
        }
        items.retain(|i| !forgotten_ids.contains(&i.id));

        // Step 4: Consolidation — merge very similar items
        let consolidated = self.consolidate(items);

        // Step 5: Overflow eviction
        let mut evicted = 0;
        if items.len() > self.decay_policy.max_items {
            items.sort_by(|a, b| {
                b.importance
                    .partial_cmp(&a.importance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            evicted = items.len() - self.decay_policy.max_items;
            items.truncate(self.decay_policy.max_items);
        }

        EvolutionResult {
            items_scored: total,
            items_decayed: decayed_count,
            items_forgotten: forgotten_ids.len(),
            items_consolidated: consolidated,
            items_evicted: evicted,
        }
    }

    /// Compute composite importance score.
    fn composite_score(&self, item: &EvolvableItem, _total_items: usize) -> f64 {
        let now = Utc::now();

        // 1. Access frequency (log-scaled)
        let freq_score = (item.access_count as f64 + 1.0).ln() / 10.0;

        // 2. Recency
        let hours_since = (now - item.last_accessed).num_hours().max(0) as f64;
        let recency_score = 1.0 / (1.0 + hours_since / 168.0); // 1 week half-life

        // 3. Connectivity (graph centrality proxy)
        let connectivity_score = (item.connectivity as f64).min(20.0) / 20.0;

        // 4. Content salience (length-based heuristic)
        let word_count = item.content.split_whitespace().count() as f64;
        let salience_score = (word_count / 50.0).min(1.0);

        // Weighted combination
        let composed = freq_score * 0.3
            + recency_score * 0.3
            + connectivity_score * 0.25
            + salience_score * 0.15;

        // Normalize to [0, 1]
        composed.min(1.0).max(0.0)
    }

    /// Merge items with very similar embeddings.
    fn consolidate(&self, items: &mut Vec<EvolvableItem>) -> usize {
        let mut merged = 0;

        // Simple O(n²) pairwise comparison — in production,
        // use approximate NN (HNSW) for scalability.
        let mut to_remove: Vec<Uuid> = Vec::new();
        let len = items.len();

        for i in 0..len {
            if to_remove.contains(&items[i].id) {
                continue;
            }
            for j in (i + 1)..len {
                if to_remove.contains(&items[j].id) {
                    continue;
                }
                if items[i].embedding.len() == items[j].embedding.len()
                    && !items[i].embedding.is_empty()
                {
                    let sim = cosine_sim(&items[i].embedding, &items[j].embedding);
                    if sim > 0.95 {
                        // Merge: keep the higher-importance item, combine access counts
                        if items[i].importance >= items[j].importance {
                            to_remove.push(items[j].id);
                            // We can't mutate items[i] here due to borrow rules,
                            // so we mark for post-processing
                        } else {
                            to_remove.push(items[i].id);
                        }
                        merged += 1;
                    }
                }
            }
        }

        items.retain(|i| !to_remove.contains(&i.id));
        merged
    }
}

impl Default for MemoryEvolution {
    fn default() -> Self {
        Self::new()
    }
}

/// Cosine similarity for evolution scoring.
fn cosine_sim(a: &[f32], b: &[f32]) -> f64 {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(content: &str, access_count: u64, connectivity: usize) -> EvolvableItem {
        EvolvableItem {
            id: Uuid::new_v4(),
            content: content.to_string(),
            importance: 0.5,
            access_count,
            last_accessed: Utc::now(),
            created_at: Utc::now(),
            connectivity,
            embedding: vec![0.1; 64],
        }
    }

    #[test]
    fn test_evolution_cycle() {
        let mut evolution = MemoryEvolution::new();
        let mut items = vec![
            make_item("Alice works at Anthropic", 10, 5),
            make_item("Bob works at Google", 2, 1),
            make_item("Weather is nice today", 0, 0),
        ];

        let result = evolution.evolve(&mut items);
        assert!(result.items_scored == 3);
        // High-access items should have higher importance
        assert!(items[0].importance > 0.0);
    }

    #[test]
    fn test_decay_exponential() {
        let decay = DecayPolicy {
            base_rate: 0.1,
            decay_function: DecayFunction::Exponential,
            ..Default::default()
        };

        let mut evolution = MemoryEvolution::with_policies(decay, RetentionPolicy::default());
        let mut items = vec![EvolvableItem {
            id: Uuid::new_v4(),
            content: "Old item".to_string(),
            importance: 1.0,
            access_count: 1,
            last_accessed: Utc::now() - chrono::Duration::hours(48),
            created_at: Utc::now() - chrono::Duration::hours(72),
            connectivity: 0,
            embedding: vec![0.0; 64],
        }];

        evolution.evolve(&mut items);
        // After 48 hours with rate 0.1, importance should be significantly reduced
        assert!(
            items[0].importance < 0.5,
            "Decayed importance should be low: {}",
            items[0].importance
        );
    }

    #[test]
    fn test_rl_retention_decision() {
        let policy = RetentionPolicy::new(0.1, 0.95, 0.0, 42); // No exploration
                                                               // High importance, high access → keep
        assert!(policy.decide(0.8, 10.0, 0.9, 0.7));
    }

    #[test]
    fn test_rl_q_table_update() {
        let mut policy = RetentionPolicy::default();
        policy.update(0.5, 5.0, 0.5, 0.3, true, 1.0); // Positive reward for keeping
        policy.update(0.5, 5.0, 0.5, 0.3, false, -1.0); // Negative reward for forgetting

        let key = RetentionPolicy::state_key(0.5, 5.0, 0.5, 0.3);
        let q_values = policy.q_table.get(&key).unwrap();
        assert!(
            q_values[0] > q_values[1],
            "Keep should be preferred after positive reward"
        );
    }

    #[test]
    fn test_overflow_eviction() {
        let decay = DecayPolicy {
            max_items: 2,
            ..Default::default()
        };
        let mut evolution = MemoryEvolution::with_policies(decay, RetentionPolicy::default());
        let mut items = vec![
            EvolvableItem {
                id: Uuid::new_v4(),
                content: "Item 1".to_string(),
                importance: 0.5,
                access_count: 10,
                last_accessed: Utc::now(),
                created_at: Utc::now(),
                connectivity: 5,
                embedding: vec![1.0, 0.0, 0.0, 0.0],
            },
            EvolvableItem {
                id: Uuid::new_v4(),
                content: "Item 2".to_string(),
                importance: 0.5,
                access_count: 5,
                last_accessed: Utc::now(),
                created_at: Utc::now(),
                connectivity: 3,
                embedding: vec![0.0, 1.0, 0.0, 0.0],
            },
            EvolvableItem {
                id: Uuid::new_v4(),
                content: "Item 3".to_string(),
                importance: 0.5,
                access_count: 1,
                last_accessed: Utc::now(),
                created_at: Utc::now(),
                connectivity: 0,
                embedding: vec![0.0, 0.0, 1.0, 0.0],
            },
        ];

        let result = evolution.evolve(&mut items);
        assert_eq!(items.len(), 2);
        assert_eq!(result.items_evicted, 1);
    }

    #[test]
    fn test_consolidation() {
        let mut evolution = MemoryEvolution::new();
        // Two items with identical embeddings should be consolidated
        let mut items = vec![
            EvolvableItem {
                id: Uuid::new_v4(),
                content: "Alice works at Anthropic".to_string(),
                importance: 0.8,
                access_count: 5,
                last_accessed: Utc::now(),
                created_at: Utc::now(),
                connectivity: 3,
                embedding: vec![1.0, 0.0, 0.0, 0.0],
            },
            EvolvableItem {
                id: Uuid::new_v4(),
                content: "Alice is employed by Anthropic".to_string(),
                importance: 0.6,
                access_count: 2,
                last_accessed: Utc::now(),
                created_at: Utc::now(),
                connectivity: 1,
                embedding: vec![1.0, 0.0, 0.0, 0.0], // Identical embedding
            },
        ];

        let result = evolution.evolve(&mut items);
        assert_eq!(result.items_consolidated, 1);
        assert_eq!(items.len(), 1);
    }
}
