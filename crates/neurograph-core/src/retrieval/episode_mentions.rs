// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Episode mentions reranker.
//!
//! Reranks results by how many episodes (provenance sources) reference
//! them. This is a proxy for "importance by citation frequency" — entities
//! mentioned across many episodes are likely more significant.
//!
//! Closes a gap where Graphiti has `EdgeReranker.episode_mentions` and
//! `NodeReranker.episode_mentions` as first-class reranking strategies.

/// Reranks items by the number of episodes that reference them.
///
/// Items with more provenance sources (episodes) are ranked higher,
/// as they represent knowledge that has been confirmed or referenced
/// across multiple ingestion events.
///
/// # Example
///
/// ```rust
/// use neurograph_core::retrieval::episode_mentions::EpisodeMentionsReranker;
///
/// // Entity A: mentioned in 5 episodes
/// // Entity B: mentioned in 1 episode
/// // After reranking: A scores 1.0, B scores 0.2
/// ```
pub struct EpisodeMentionsReranker;

impl EpisodeMentionsReranker {
    /// Rerank items by their episode mention counts.
    ///
    /// Scores are normalized: most-mentioned = 1.0, others proportional.
    /// Items with zero mentions get score 0.0.
    pub fn rerank<T: HasEpisodeIds + Clone>(items: &[(T, f32)]) -> Vec<(T, f32)> {
        if items.is_empty() {
            return Vec::new();
        }

        let max_mentions = items
            .iter()
            .map(|(item, _)| item.episode_count())
            .max()
            .unwrap_or(1)
            .max(1) as f32;

        let mut scored: Vec<(T, f32)> = items
            .iter()
            .map(|(item, _original_score)| {
                let mention_score = item.episode_count() as f32 / max_mentions;
                (item.clone(), mention_score)
            })
            .collect();

        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        scored
    }

    /// Combine episode mention count with an existing relevance score.
    ///
    /// Combined = α · relevance_score + (1-α) · normalized_mention_count
    pub fn combine_scores<T: HasEpisodeIds>(
        items: &[(T, f32)],
        alpha: f32,
    ) -> Vec<f32> {
        if items.is_empty() {
            return Vec::new();
        }

        let max_mentions = items
            .iter()
            .map(|(item, _)| item.episode_count())
            .max()
            .unwrap_or(1)
            .max(1) as f32;

        items
            .iter()
            .map(|(item, relevance)| {
                let mention_score = item.episode_count() as f32 / max_mentions;
                alpha * relevance + (1.0 - alpha) * mention_score
            })
            .collect()
    }
}

/// Trait for items that track episode provenance.
pub trait HasEpisodeIds {
    /// Get the episode IDs that reference this item.
    fn episode_ids(&self) -> &[String];

    /// Count of episodes referencing this item.
    fn episode_count(&self) -> usize {
        self.episode_ids().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct TestItem {
        name: String,
        episodes: Vec<String>,
    }

    impl HasEpisodeIds for TestItem {
        fn episode_ids(&self) -> &[String] {
            &self.episodes
        }
    }

    #[test]
    fn test_rerank_by_mentions() {
        let items = vec![
            (
                TestItem {
                    name: "few_mentions".into(),
                    episodes: vec!["ep1".into()],
                },
                0.9, // High relevance but few mentions
            ),
            (
                TestItem {
                    name: "many_mentions".into(),
                    episodes: vec!["ep1".into(), "ep2".into(), "ep3".into(), "ep4".into(), "ep5".into()],
                },
                0.5, // Lower relevance but many mentions
            ),
        ];

        let reranked = EpisodeMentionsReranker::rerank(&items);
        assert_eq!(reranked[0].0.name, "many_mentions"); // Most mentions first
        assert!((reranked[0].1 - 1.0).abs() < f32::EPSILON); // Normalized to 1.0
        assert!((reranked[1].1 - 0.2).abs() < f32::EPSILON); // 1/5 = 0.2
    }

    #[test]
    fn test_combine_scores() {
        let items = vec![
            (
                TestItem {
                    name: "A".into(),
                    episodes: vec!["ep1".into(), "ep2".into()],
                },
                0.9,
            ),
            (
                TestItem {
                    name: "B".into(),
                    episodes: vec!["ep1".into(), "ep2".into(), "ep3".into(), "ep4".into()],
                },
                0.5,
            ),
        ];

        // α = 0.5 → equal weight to relevance and mentions
        let combined = EpisodeMentionsReranker::combine_scores(&items, 0.5);
        assert_eq!(combined.len(), 2);

        // A: 0.5 * 0.9 + 0.5 * (2/4) = 0.45 + 0.25 = 0.70
        assert!((combined[0] - 0.70).abs() < 0.01);
        // B: 0.5 * 0.5 + 0.5 * (4/4) = 0.25 + 0.50 = 0.75
        assert!((combined[1] - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_empty_items() {
        let items: Vec<(TestItem, f32)> = vec![];
        let reranked = EpisodeMentionsReranker::rerank(&items);
        assert!(reranked.is_empty());
    }
}
