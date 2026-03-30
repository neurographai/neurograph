// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Tiered Memory System with L1–L4 hierarchy.
//!
//! - **L1 (Working)**: Current context window, hot cache, max ~100 items
//! - **L2 (Episodic)**: Recent interactions, indexed by time, max ~10,000 items
//! - **L3 (Semantic)**: Knowledge graph facts (core entity graph)
//! - **L4 (Procedural)**: Learned patterns, heuristics, consolidated knowledge
//!
//! Items are automatically promoted/demoted based on access patterns,
//! importance scores, and configurable policies.
//!
//! Reference: "EverMemOS Tiered Memory" — L1/L2/L3/L4 hierarchy (2026).

use std::collections::{HashMap, VecDeque};
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};

use crate::multigraph::MemoryTier;

/// An item stored in the tiered memory system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieredItem {
    pub id: Uuid,
    pub content: String,
    pub tier: MemoryTier,
    pub importance: f64,
    pub access_count: u64,
    pub last_accessed: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub promoted_at: Option<DateTime<Utc>>,
    pub decay_score: f64,
    pub tags: Vec<String>,
}

/// Configuration for automatic consolidation (merging similar L2 items into L3).
#[derive(Debug, Clone)]
pub struct ConsolidationConfig {
    /// Minimum similarity threshold for merging items.
    pub similarity_threshold: f64,
    /// Minimum number of similar items required to consolidate.
    pub min_cluster_size: usize,
    /// How often consolidation runs (in seconds).
    pub interval_secs: u64,
    /// Maximum items to process per consolidation cycle.
    pub batch_size: usize,
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.85,
            min_cluster_size: 3,
            interval_secs: 3600,
            batch_size: 100,
        }
    }
}

/// Policy for automatic tier promotion/demotion.
#[derive(Debug, Clone)]
pub struct PromotionPolicy {
    /// Working → Episodic: after N accesses
    pub l1_to_l2_access_threshold: u64,
    /// Episodic → Semantic: after N accesses AND importance > threshold
    pub l2_to_l3_access_threshold: u64,
    pub l2_to_l3_importance_threshold: f64,
    /// Semantic → Procedural: after N accesses AND confirmed by pattern detection
    pub l3_to_l4_access_threshold: u64,
    /// Demotion: if not accessed for N hours, demote one tier
    pub demotion_hours: i64,
    /// Maximum items per tier
    pub l1_capacity: usize,
    pub l2_capacity: usize,
}

impl Default for PromotionPolicy {
    fn default() -> Self {
        Self {
            l1_to_l2_access_threshold: 3,
            l2_to_l3_access_threshold: 10,
            l2_to_l3_importance_threshold: 0.6,
            l3_to_l4_access_threshold: 50,
            demotion_hours: 168, // 1 week
            l1_capacity: 100,
            l2_capacity: 10_000,
        }
    }
}

/// The tiered memory system.
pub struct TieredMemory {
    /// L1: Working memory (hot, small)
    l1_working: VecDeque<TieredItem>,
    /// L2: Episodic memory (recent, medium)
    l2_episodic: Vec<TieredItem>,
    /// L2 grouped episodes (interaction-grouped memories)
    episodes: Vec<Episode>,
    /// L3: Semantic memory (knowledge graph facts, large)
    l3_semantic: Vec<TieredItem>,
    /// L4: Procedural memory (patterns, heuristics)
    l4_procedural: Vec<TieredItem>,
    /// L4 learned rules (condensed patterns)
    rules: Vec<LearnedRule>,
    /// Quick lookup: id -> tier
    index: HashMap<Uuid, MemoryTier>,
    /// Promotion/demotion policy
    policy: PromotionPolicy,
    /// Consolidation config
    #[allow(dead_code)]
    consolidation: ConsolidationConfig,
}

/// A grouped episode in L2 episodic memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: Uuid,
    /// Memory item IDs belonging to this episode.
    pub item_ids: Vec<Uuid>,
    /// LLM- or extraction-generated summary of the episode.
    pub summary: String,
    /// When this episode occurred.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Importance score for the episode.
    pub importance: f64,
}

/// A learned rule in L4 procedural memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedRule {
    pub id: Uuid,
    /// The pattern or rule text (e.g., "When user asks about X, include Y").
    pub pattern: String,
    /// Confidence in the rule (0.0–1.0).
    pub confidence: f64,
    /// How many times this rule has been applied.
    pub usage_count: u64,
    /// Source memory IDs that led to learning this rule.
    pub source_ids: Vec<Uuid>,
    /// When this rule was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Aggregated context from all tiers for a query.
#[derive(Debug)]
pub struct QueryContext {
    /// L1 working memory items (full context).
    pub working_items: Vec<Uuid>,
    /// L2 recent episode IDs.
    pub recent_episodes: Vec<Uuid>,
    /// L4 applicable procedural rules.
    pub applicable_rules: Vec<LearnedRule>,
}

/// Stats for the tiered memory system.
#[derive(Debug, Clone)]
pub struct TieredStats {
    pub l1_count: usize,
    pub l2_count: usize,
    pub l3_count: usize,
    pub l4_count: usize,
    pub total_accesses: u64,
    pub promotions: u64,
    pub demotions: u64,
}

impl TieredMemory {
    /// Create a new tiered memory with default policies.
    pub fn new() -> Self {
        Self::with_policies(PromotionPolicy::default(), ConsolidationConfig::default())
    }

    /// Create with custom policies.
    pub fn with_policies(policy: PromotionPolicy, consolidation: ConsolidationConfig) -> Self {
        Self {
            l1_working: VecDeque::with_capacity(policy.l1_capacity),
            l2_episodic: Vec::new(),
            episodes: Vec::new(),
            l3_semantic: Vec::new(),
            l4_procedural: Vec::new(),
            rules: Vec::new(),
            index: HashMap::new(),
            policy,
            consolidation,
        }
    }

    /// Remember a new piece of information. Enters at L1 (Working).
    pub fn remember(&mut self, content: String) -> Uuid {
        let now = Utc::now();
        let id = Uuid::new_v4();

        let item = TieredItem {
            id,
            content,
            tier: MemoryTier::Working,
            importance: 0.5,
            access_count: 1,
            last_accessed: now,
            created_at: now,
            promoted_at: None,
            decay_score: 1.0,
            tags: Vec::new(),
        };

        // Evict oldest if L1 is full
        if self.l1_working.len() >= self.policy.l1_capacity {
            if let Some(evicted) = self.l1_working.pop_front() {
                // Demote evicted item to L2
                self.index.insert(evicted.id, MemoryTier::Episodic);
                self.l2_episodic.push(TieredItem {
                    tier: MemoryTier::Episodic,
                    ..evicted
                });
            }
        }

        self.index.insert(id, MemoryTier::Working);
        self.l1_working.push_back(item);
        id
    }

    /// Access (recall) a memory item, updating access stats and potentially promoting.
    pub fn access(&mut self, id: Uuid) -> Option<&TieredItem> {
        let now = Utc::now();

        // Find and update the item
        if let Some(tier) = self.index.get(&id).copied() {
            match tier {
                MemoryTier::Working => {
                    if let Some(item) = self.l1_working.iter_mut().find(|i| i.id == id) {
                        item.access_count += 1;
                        item.last_accessed = now;
                        self.check_promotion(id);
                        return self.l1_working.iter().find(|i| i.id == id);
                    }
                }
                MemoryTier::Episodic => {
                    if let Some(item) = self.l2_episodic.iter_mut().find(|i| i.id == id) {
                        item.access_count += 1;
                        item.last_accessed = now;
                        self.check_promotion(id);
                        return self.l2_episodic.iter().find(|i| i.id == id);
                    }
                }
                MemoryTier::Semantic => {
                    if let Some(item) = self.l3_semantic.iter_mut().find(|i| i.id == id) {
                        item.access_count += 1;
                        item.last_accessed = now;
                        self.check_promotion(id);
                        return self.l3_semantic.iter().find(|i| i.id == id);
                    }
                }
                MemoryTier::Procedural => {
                    if let Some(item) = self.l4_procedural.iter_mut().find(|i| i.id == id) {
                        item.access_count += 1;
                        item.last_accessed = now;
                        return self.l4_procedural.iter().find(|i| i.id == id);
                    }
                }
            }
        }

        None
    }

    /// Search across all tiers for matching content.
    pub fn search(&self, query: &str, max_results: usize) -> Vec<&TieredItem> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<(&TieredItem, f64)> = Vec::new();

        // Search L1 first (hot cache, highest tier boost)
        for item in &self.l1_working {
            let score = text_match_score(&item.content, &query_lower) * 4.0;
            if score > 0.0 {
                results.push((item, score));
            }
        }

        // Search L2
        for item in &self.l2_episodic {
            let score = text_match_score(&item.content, &query_lower) * 2.0;
            if score > 0.0 {
                results.push((item, score));
            }
        }

        // Search L3
        for item in &self.l3_semantic {
            let score = text_match_score(&item.content, &query_lower) * 1.5;
            if score > 0.0 {
                results.push((item, score));
            }
        }

        // Search L4
        for item in &self.l4_procedural {
            let score = text_match_score(&item.content, &query_lower) * 1.0;
            if score > 0.0 {
                results.push((item, score));
            }
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(max_results);
        results.into_iter().map(|(item, _)| item).collect()
    }

    /// Run promotion/demotion checks on all items.
    pub fn maintain(&mut self) -> MaintenanceReport {
        let now = Utc::now();
        let mut promotions = 0u64;
        let mut demotions = 0u64;

        // L1 → L2 promotion (based on access count)
        let mut to_promote_l1: Vec<Uuid> = Vec::new();
        for item in &self.l1_working {
            if item.access_count >= self.policy.l1_to_l2_access_threshold {
                to_promote_l1.push(item.id);
            }
        }
        for id in to_promote_l1 {
            self.promote(id, MemoryTier::Episodic);
            promotions += 1;
        }

        // L2 → L3 promotion
        let mut to_promote_l2: Vec<Uuid> = Vec::new();
        for item in &self.l2_episodic {
            if item.access_count >= self.policy.l2_to_l3_access_threshold
                && item.importance >= self.policy.l2_to_l3_importance_threshold
            {
                to_promote_l2.push(item.id);
            }
        }
        for id in to_promote_l2 {
            self.promote(id, MemoryTier::Semantic);
            promotions += 1;
        }

        // Demotion: items not accessed within demotion window
        let demotion_cutoff = now - Duration::hours(self.policy.demotion_hours);

        let mut to_demote_l2: Vec<Uuid> = Vec::new();
        for item in &self.l2_episodic {
            if item.last_accessed < demotion_cutoff && item.importance < 0.3 {
                to_demote_l2.push(item.id);
            }
        }
        // Remove low-importance expired L2 items (eviction, not demotion)
        for id in &to_demote_l2 {
            self.l2_episodic.retain(|i| i.id != *id);
            self.index.remove(id);
            demotions += 1;
        }

        // L2 overflow: push oldest items out if over capacity
        while self.l2_episodic.len() > self.policy.l2_capacity {
            if let Some(oldest_idx) = self
                .l2_episodic
                .iter()
                .enumerate()
                .min_by_key(|(_, i)| i.last_accessed)
                .map(|(idx, _)| idx)
            {
                let evicted = self.l2_episodic.remove(oldest_idx);
                self.index.remove(&evicted.id);
                demotions += 1;
            } else {
                break;
            }
        }

        MaintenanceReport {
            promotions,
            demotions,
            l1_count: self.l1_working.len(),
            l2_count: self.l2_episodic.len(),
            l3_count: self.l3_semantic.len(),
            l4_count: self.l4_procedural.len(),
        }
    }

    /// Get statistics.
    pub fn stats(&self) -> TieredStats {
        let total_accesses: u64 = self
            .l1_working.iter().map(|i| i.access_count)
            .chain(self.l2_episodic.iter().map(|i| i.access_count))
            .chain(self.l3_semantic.iter().map(|i| i.access_count))
            .chain(self.l4_procedural.iter().map(|i| i.access_count))
            .sum();

        TieredStats {
            l1_count: self.l1_working.len(),
            l2_count: self.l2_episodic.len(),
            l3_count: self.l3_semantic.len(),
            l4_count: self.l4_procedural.len(),
            total_accesses,
            promotions: 0,
            demotions: 0,
        }
    }

    /// Total items across all tiers.
    pub fn total_items(&self) -> usize {
        self.l1_working.len()
            + self.l2_episodic.len()
            + self.l3_semantic.len()
            + self.l4_procedural.len()
    }

    fn promote(&mut self, id: Uuid, target_tier: MemoryTier) {
        let now = Utc::now();

        // Find and remove from current tier
        let item = self.remove_item(id);
        if let Some(mut item) = item {
            item.tier = target_tier;
            item.promoted_at = Some(now);
            self.index.insert(id, target_tier);

            match target_tier {
                MemoryTier::Working => self.l1_working.push_back(item),
                MemoryTier::Episodic => self.l2_episodic.push(item),
                MemoryTier::Semantic => self.l3_semantic.push(item),
                MemoryTier::Procedural => self.l4_procedural.push(item),
            }
        }
    }

    fn remove_item(&mut self, id: Uuid) -> Option<TieredItem> {
        // Check L1
        if let Some(pos) = self.l1_working.iter().position(|i| i.id == id) {
            return self.l1_working.remove(pos);
        }
        // Check L2
        if let Some(pos) = self.l2_episodic.iter().position(|i| i.id == id) {
            return Some(self.l2_episodic.remove(pos));
        }
        // Check L3
        if let Some(pos) = self.l3_semantic.iter().position(|i| i.id == id) {
            return Some(self.l3_semantic.remove(pos));
        }
        // Check L4
        if let Some(pos) = self.l4_procedural.iter().position(|i| i.id == id) {
            return Some(self.l4_procedural.remove(pos));
        }
        None
    }

    fn check_promotion(&mut self, id: Uuid) {
        if let Some(tier) = self.index.get(&id).copied() {
            match tier {
                MemoryTier::Working => {
                    if let Some(item) = self.l1_working.iter().find(|i| i.id == id) {
                        if item.access_count >= self.policy.l1_to_l2_access_threshold {
                            self.promote(id, MemoryTier::Episodic);
                        }
                    }
                }
                MemoryTier::Episodic => {
                    if let Some(item) = self.l2_episodic.iter().find(|i| i.id == id) {
                        if item.access_count >= self.policy.l2_to_l3_access_threshold
                            && item.importance >= self.policy.l2_to_l3_importance_threshold
                        {
                            self.promote(id, MemoryTier::Semantic);
                        }
                    }
                }
                MemoryTier::Semantic => {
                    if let Some(item) = self.l3_semantic.iter().find(|i| i.id == id) {
                        if item.access_count >= self.policy.l3_to_l4_access_threshold {
                            self.promote(id, MemoryTier::Procedural);
                        }
                    }
                }
                MemoryTier::Procedural => {} // Top tier, no further promotion
            }
        }
    }
}

impl Default for TieredMemory {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Episode & Rule methods ─────────────────────────────────

impl TieredMemory {
    /// Ingest a grouped episode into L2.
    pub fn ingest_episode(&mut self, summary: String, item_ids: Vec<Uuid>) -> Uuid {
        let id = Uuid::new_v4();
        self.episodes.push(Episode {
            id,
            item_ids,
            summary,
            timestamp: Utc::now(),
            importance: 0.5,
        });
        id
    }

    /// Get recent episodes (newest first).
    pub fn recent_episodes(&self, n: usize) -> Vec<&Episode> {
        self.episodes.iter().rev().take(n).collect()
    }

    /// Get episodes in a time range.
    pub fn episodes_in_range(
        &self,
        from: chrono::DateTime<chrono::Utc>,
        to: chrono::DateTime<chrono::Utc>,
    ) -> Vec<&Episode> {
        self.episodes
            .iter()
            .filter(|e| e.timestamp >= from && e.timestamp <= to)
            .collect()
    }

    /// Add a learned rule to L4 procedural memory.
    pub fn add_rule(
        &mut self,
        pattern: String,
        confidence: f64,
        source_ids: Vec<Uuid>,
    ) -> Uuid {
        let id = Uuid::new_v4();
        self.rules.push(LearnedRule {
            id,
            pattern,
            confidence,
            usage_count: 0,
            source_ids,
            created_at: Utc::now(),
        });
        id
    }

    /// Find rules whose pattern matches the query (simple keyword overlap).
    pub fn find_applicable_rules(&self, query: &str) -> Vec<LearnedRule> {
        let query_lower = query.to_lowercase();
        self.rules
            .iter()
            .filter(|r| {
                let pattern_lower = r.pattern.to_lowercase();
                query_lower.split_whitespace().any(|w| pattern_lower.contains(w))
            })
            .cloned()
            .collect()
    }

    /// Get aggregated context from all tiers for a query.
    ///
    /// Returns L1 working memory (full), recent L2 episodes, and applicable L4 rules.
    pub fn get_context_for_query(&self, query: &str) -> QueryContext {
        let working_items: Vec<Uuid> = self.l1_working.iter().map(|i| i.id).collect();
        let recent_episodes: Vec<Uuid> = self.episodes.iter().rev().take(10).map(|e| e.id).collect();
        let applicable_rules = self.find_applicable_rules(query);

        QueryContext {
            working_items,
            recent_episodes,
            applicable_rules,
        }
    }

    /// Number of episodes.
    pub fn episode_count(&self) -> usize {
        self.episodes.len()
    }

    /// Number of learned rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

/// Result of a maintenance cycle.
#[derive(Debug)]
pub struct MaintenanceReport {
    pub promotions: u64,
    pub demotions: u64,
    pub l1_count: usize,
    pub l2_count: usize,
    pub l3_count: usize,
    pub l4_count: usize,
}

/// Simple word-overlap text match score.
fn text_match_score(text: &str, query: &str) -> f64 {
    let text_lower = text.to_lowercase();
    let query_words: Vec<&str> = query.split_whitespace().filter(|w| w.len() > 1).collect();
    if query_words.is_empty() {
        return 0.0;
    }
    let matches = query_words
        .iter()
        .filter(|w| text_lower.contains(**w))
        .count();
    matches as f64 / query_words.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remember_and_search() {
        let mut mem = TieredMemory::new();
        mem.remember("Alice works at Anthropic".to_string());
        mem.remember("Bob works at Google".to_string());

        let results = mem.search("Alice Anthropic", 10);
        assert!(!results.is_empty());
        assert!(results[0].content.contains("Alice"));
    }

    #[test]
    fn test_l1_capacity_eviction() {
        let mut mem = TieredMemory::with_policies(
            PromotionPolicy {
                l1_capacity: 3,
                ..Default::default()
            },
            ConsolidationConfig::default(),
        );

        mem.remember("Item 1".to_string());
        mem.remember("Item 2".to_string());
        mem.remember("Item 3".to_string());
        assert_eq!(mem.stats().l1_count, 3);

        // Adding a 4th should evict the oldest to L2
        mem.remember("Item 4".to_string());
        assert_eq!(mem.stats().l1_count, 3);
        assert_eq!(mem.stats().l2_count, 1);
    }

    #[test]
    fn test_access_count_tracking() {
        let mut mem = TieredMemory::new();
        let id = mem.remember("Test item".to_string());

        mem.access(id);
        mem.access(id);
        let item = mem.access(id);
        assert!(item.is_some());
        // Initial + 3 accesses = 4
        assert_eq!(item.unwrap().access_count, 4);
    }

    #[test]
    fn test_total_items() {
        let mut mem = TieredMemory::new();
        mem.remember("Item 1".to_string());
        mem.remember("Item 2".to_string());
        assert_eq!(mem.total_items(), 2);
    }

    #[test]
    fn test_search_prioritizes_higher_tiers() {
        let mut mem = TieredMemory::with_policies(
            PromotionPolicy {
                l1_capacity: 2,
                ..Default::default()
            },
            ConsolidationConfig::default(),
        );

        // This will be evicted to L2 when 2 more items arrive
        let _id1 = mem.remember("Alice researcher knowledge".to_string());
        let _id2 = mem.remember("Filler item".to_string());
        let _id3 = mem.remember("Alice engineer systems".to_string());
        // id1 should now be in L2, id3 in L1

        let results = mem.search("Alice", 10);
        assert!(results.len() >= 2);
        // L1 item should rank first (4x tier boost)
        assert!(results[0].content.contains("engineer") || results[0].tier == MemoryTier::Working);
    }
}
