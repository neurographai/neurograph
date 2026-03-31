// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Multi-channel search results with per-item scores.
//!
//! Closes a critical gap where Graphiti returns typed `SearchResults`
//! with separate lists and reranker scores for edges, nodes, episodes,
//! and communities — while NeuroGraph returned a flat entity list.
//!
//! Each channel is independently searched and reranked, then results
//! can be flattened into a single ranked list for backward compatibility.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

use crate::graph::{Community, Entity, Episode, Relationship};

// ─── Scored Item ──────────────────────────────────────────────────────

/// A search result item with attached scores and provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredItem<T> {
    /// The item found.
    pub item: T,
    /// Initial score from the search method (e.g., cosine similarity).
    pub score: f32,
    /// Which search method produced the initial hit.
    pub source_method: String,
    /// Score from the reranker (may differ from initial score).
    pub reranker_score: Option<f32>,
}

impl<T> ScoredItem<T> {
    /// Create a new scored item.
    pub fn new(item: T, score: f32, source: impl Into<String>) -> Self {
        Self {
            item,
            score,
            source_method: source.into(),
            reranker_score: None,
        }
    }

    /// Attach a reranker score.
    pub fn with_reranker_score(mut self, score: f32) -> Self {
        self.reranker_score = Some(score);
        self
    }

    /// Get the effective score (reranker score if available, otherwise original).
    pub fn effective_score(&self) -> f32 {
        self.reranker_score.unwrap_or(self.score)
    }
}

// ─── Search Metadata ──────────────────────────────────────────────────

/// Metadata about the search execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchMetadata {
    /// Original query string.
    pub query: String,
    /// Name of the search config/recipe used.
    pub config_name: Option<String>,
    /// Total candidates considered before reranking.
    pub total_candidates_considered: usize,
    /// Total wall-clock time for the search.
    pub search_duration_ms: u64,
    /// Which channels were searched.
    pub channels_searched: Vec<String>,
}

// ─── Multi-Channel Search Results ─────────────────────────────────────

/// Typed, multi-channel search results with per-item scores.
///
/// Each channel is independently searched and reranked. This is the
/// Rust equivalent of Graphiti's `SearchResults` which carries separate
/// lists for edges, nodes, episodes, and communities.
///
/// Use `.flatten()` for backward-compatible single-list output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchResults {
    /// Entity (node) results with scores.
    pub entities: Vec<ScoredItem<Entity>>,
    /// Relationship (edge) results with scores.
    pub relationships: Vec<ScoredItem<Relationship>>,
    /// Episode (provenance) results with scores.
    pub episodes: Vec<ScoredItem<Episode>>,
    /// Community (cluster) results with scores.
    pub communities: Vec<ScoredItem<Community>>,
    /// Execution metadata.
    pub metadata: SearchMetadata,
}

/// Channel type for flattened results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Channel {
    /// Entity (node) channel.
    Entity,
    /// Relationship (edge) channel.
    Relationship,
    /// Episode (provenance) channel.
    Episode,
    /// Community (cluster) channel.
    Community,
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Channel::Entity => write!(f, "entity"),
            Channel::Relationship => write!(f, "relationship"),
            Channel::Episode => write!(f, "episode"),
            Channel::Community => write!(f, "community"),
        }
    }
}

/// A flattened search result with channel info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatResult {
    /// Identifier of the item.
    pub id: String,
    /// Display name of the item.
    pub name: String,
    /// Which channel produced this result.
    pub channel: Channel,
    /// Effective score (from reranker if available).
    pub score: f32,
    /// Short text snippet for display.
    pub snippet: String,
}

impl SearchResults {
    /// Create empty search results.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Flatten all results into a single ranked list (backward compatibility).
    ///
    /// Interleaves channels by effective score descending. This allows
    /// existing code that expects `Vec<Entity>` to continue working.
    pub fn flatten(&self) -> Vec<FlatResult> {
        let mut all: Vec<FlatResult> = Vec::with_capacity(self.total_results());

        for item in &self.entities {
            all.push(FlatResult {
                id: item.item.id.as_str(),
                name: item.item.name.clone(),
                channel: Channel::Entity,
                score: item.effective_score(),
                snippet: if item.item.summary.is_empty() {
                    item.item.entity_type.0.clone()
                } else {
                    truncate_str(&item.item.summary, 200)
                },
            });
        }

        for item in &self.relationships {
            all.push(FlatResult {
                id: item.item.id.as_str(),
                name: item.item.relationship_type.clone(),
                channel: Channel::Relationship,
                score: item.effective_score(),
                snippet: item.item.fact.clone(),
            });
        }

        for item in &self.episodes {
            all.push(FlatResult {
                id: item.item.id.as_str(),
                name: item.item.name.clone(),
                channel: Channel::Episode,
                score: item.effective_score(),
                snippet: truncate_str(&item.item.content, 200),
            });
        }

        for item in &self.communities {
            all.push(FlatResult {
                id: item.item.id.as_str().to_string(),
                name: item.item.name.clone(),
                channel: Channel::Community,
                score: item.effective_score(),
                snippet: item.item.summary.clone(),
            });
        }

        all.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(Ordering::Equal)
        });

        all
    }

    /// Get the total number of results across all channels.
    pub fn total_results(&self) -> usize {
        self.entities.len()
            + self.relationships.len()
            + self.episodes.len()
            + self.communities.len()
    }

    /// Check if all channels are empty.
    pub fn is_empty(&self) -> bool {
        self.total_results() == 0
    }

    /// Get results from a specific channel as flat results.
    pub fn channel_results(&self, channel: Channel) -> Vec<FlatResult> {
        self.flatten()
            .into_iter()
            .filter(|r| r.channel == channel)
            .collect()
    }

    /// Merge two SearchResults together (union of all channels).
    pub fn merge(mut self, other: SearchResults) -> Self {
        self.entities.extend(other.entities);
        self.relationships.extend(other.relationships);
        self.episodes.extend(other.episodes);
        self.communities.extend(other.communities);
        self.metadata.total_candidates_considered +=
            other.metadata.total_candidates_considered;
        self.metadata.channels_searched.extend(
            other.metadata.channels_searched,
        );
        self
    }

    /// Get top-k entities only (convenience for backward compatibility).
    pub fn top_entities(&self, k: usize) -> Vec<&Entity> {
        let mut sorted: Vec<_> = self.entities.iter().collect();
        sorted.sort_by(|a, b| {
            b.effective_score()
                .partial_cmp(&a.effective_score())
                .unwrap_or(Ordering::Equal)
        });
        sorted.into_iter().take(k).map(|s| &s.item).collect()
    }

    /// Get top-k relationships only.
    pub fn top_relationships(&self, k: usize) -> Vec<&Relationship> {
        let mut sorted: Vec<_> = self.relationships.iter().collect();
        sorted.sort_by(|a, b| {
            b.effective_score()
                .partial_cmp(&a.effective_score())
                .unwrap_or(Ordering::Equal)
        });
        sorted.into_iter().take(k).map(|s| &s.item).collect()
    }
}

/// Truncate a string to a maximum length, adding "…" if truncated.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Entity;

    #[test]
    fn test_empty_results() {
        let results = SearchResults::empty();
        assert!(results.is_empty());
        assert_eq!(results.total_results(), 0);
        assert!(results.flatten().is_empty());
    }

    #[test]
    fn test_scored_item_effective_score() {
        let item = ScoredItem::new("test", 0.5, "cosine");
        assert!((item.effective_score() - 0.5).abs() < f32::EPSILON);

        let item2 = item.with_reranker_score(0.8);
        assert!((item2.effective_score() - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_flatten_sorts_by_score() {
        let mut results = SearchResults::empty();

        let e1 = Entity::new("Low", "Type");
        let e2 = Entity::new("High", "Type");

        results.entities.push(ScoredItem::new(e1, 0.3, "cosine"));
        results.entities.push(ScoredItem::new(e2, 0.9, "cosine"));

        let flat = results.flatten();
        assert_eq!(flat.len(), 2);
        assert_eq!(flat[0].name, "High");
        assert_eq!(flat[1].name, "Low");
    }

    #[test]
    fn test_merge() {
        let mut r1 = SearchResults::empty();
        r1.entities.push(ScoredItem::new(
            Entity::new("A", "Type"),
            0.5,
            "cosine",
        ));

        let mut r2 = SearchResults::empty();
        r2.entities.push(ScoredItem::new(
            Entity::new("B", "Type"),
            0.7,
            "bm25",
        ));

        let merged = r1.merge(r2);
        assert_eq!(merged.entities.len(), 2);
    }

    #[test]
    fn test_top_entities() {
        let mut results = SearchResults::empty();
        results.entities.push(ScoredItem::new(
            Entity::new("Low", "T"),
            0.3,
            "cosine",
        ));
        results.entities.push(ScoredItem::new(
            Entity::new("High", "T"),
            0.9,
            "cosine",
        ));

        let top = results.top_entities(1);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].name, "High");
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("short", 10), "short");
        assert_eq!(truncate_str("this is a long string", 10), "this is a…");
    }
}
