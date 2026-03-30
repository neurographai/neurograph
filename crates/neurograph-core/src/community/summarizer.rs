// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Community summarization using LLM or rule-based approaches.
//!
//! Generates human-readable summaries for detected communities.
//! These summaries are used by the global query strategy
//! (map-reduce over community descriptions).
//!
//! Influenced by GraphRAG's community_reports generation
//! which uses LLM to summarize community contents.

use std::sync::Arc;

use crate::drivers::traits::GraphDriver;
use crate::graph::Community;
use crate::llm::traits::LlmClient;

/// Result of summarizing a community.
#[derive(Debug, Clone)]
pub struct CommunitySummaryResult {
    /// The community that was summarized.
    pub community_id: String,
    /// The generated summary.
    pub summary: String,
    /// LLM cost for generating this summary (USD).
    pub cost_usd: f64,
}

/// Generates summaries for communities.
pub struct CommunitySummarizer {
    driver: Arc<dyn GraphDriver>,
    llm: Option<Arc<dyn LlmClient>>,
}

impl CommunitySummarizer {
    /// Create a new summarizer.
    pub fn new(driver: Arc<dyn GraphDriver>, llm: Option<Arc<dyn LlmClient>>) -> Self {
        Self { driver, llm }
    }

    /// Generate a summary for a single community.
    ///
    /// If an LLM is available, uses it to create a rich narrative summary.
    /// Otherwise, falls back to rule-based summarization (entity list + types).
    pub async fn summarize_community(
        &self,
        community: &Community,
    ) -> Result<CommunitySummaryResult, SummarizerError> {
        // Get member entities
        let mut member_entities = Vec::new();
        for member_id in community.members() {
            if let Ok(entity) = self.driver.get_entity(member_id).await {
                member_entities.push(entity);
            }
        }

        if member_entities.is_empty() {
            return Ok(CommunitySummaryResult {
                community_id: community.id.as_str().to_string(),
                summary: String::new(),
                cost_usd: 0.0,
            });
        }

        // Get relationships between members
        let member_ids: std::collections::HashSet<String> = member_entities
            .iter()
            .map(|e| e.id.as_str())
            .collect();

        let mut internal_facts = Vec::new();
        for entity in &member_entities {
            let rels = self
                .driver
                .get_entity_relationships(&entity.id)
                .await
                .unwrap_or_default();

            for rel in &rels {
                if member_ids.contains(&rel.source_entity_id.as_str())
                    && member_ids.contains(&rel.target_entity_id.as_str())
                    && rel.is_valid()
                {
                    internal_facts.push(rel.fact.clone());
                }
            }
        }

        // Deduplicate facts
        let mut seen = std::collections::HashSet::new();
        internal_facts.retain(|f| seen.insert(f.clone()));

        // Try LLM summarization, fall back to rule-based
        if let Some(ref llm) = self.llm {
            match self.summarize_with_llm(llm.as_ref(), &member_entities, &internal_facts).await {
                Ok(result) => return Ok(result.with_community_id(community.id.as_str().to_string())),
                Err(e) => {
                    tracing::warn!(error = %e, "LLM summarization failed, using rule-based");
                }
            }
        }

        // Rule-based fallback
        Ok(self.summarize_rule_based(community, &member_entities, &internal_facts))
    }

    /// LLM-based community summarization.
    async fn summarize_with_llm(
        &self,
        llm: &dyn LlmClient,
        entities: &[crate::graph::Entity],
        facts: &[String],
    ) -> Result<CommunitySummaryResult, SummarizerError> {
        use crate::llm::traits::CompletionRequest;

        let entity_desc: Vec<String> = entities
            .iter()
            .map(|e| {
                if e.summary.is_empty() {
                    format!("- {} ({})", e.name, e.entity_type)
                } else {
                    format!("- {} ({}): {}", e.name, e.entity_type, e.summary)
                }
            })
            .collect();

        let facts_desc: Vec<String> = facts.iter().map(|f| format!("- {}", f)).collect();

        let prompt = format!(
            "Summarize this community of entities in 2-3 sentences:\n\n\
             Entities:\n{}\n\n\
             Known Facts:\n{}\n\n\
             Write a concise summary describing what this community is about, \
             the key entities, and their relationships.",
            entity_desc.join("\n"),
            if facts_desc.is_empty() {
                "No known relationships.".to_string()
            } else {
                facts_desc.join("\n")
            }
        );

        let request = CompletionRequest::new(prompt)
            .with_system("You are a knowledge graph analyst. Write concise community summaries.")
            .with_temperature(0.3)
            .with_max_tokens(200);

        let response = llm
            .complete(request)
            .await
            .map_err(|e| SummarizerError::LlmError(e.to_string()))?;

        Ok(CommunitySummaryResult {
            community_id: String::new(),
            summary: response.content,
            cost_usd: response.usage.cost_usd,
        })
    }

    /// Rule-based community summarization (no LLM required).
    fn summarize_rule_based(
        &self,
        community: &Community,
        entities: &[crate::graph::Entity],
        facts: &[String],
    ) -> CommunitySummaryResult {
        // Group entities by type
        let mut by_type: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for entity in entities {
            by_type
                .entry(entity.entity_type.as_str().to_string())
                .or_default()
                .push(entity.name.clone());
        }

        let mut parts = Vec::new();

        // Describe entity composition
        let type_descriptions: Vec<String> = by_type
            .iter()
            .map(|(t, names)| {
                if names.len() <= 3 {
                    format!("{} ({})", names.join(", "), t)
                } else {
                    format!("{} and {} more ({})", names[..2].join(", "), names.len() - 2, t)
                }
            })
            .collect();

        if !type_descriptions.is_empty() {
            parts.push(format!(
                "A community of {} entities: {}.",
                entities.len(),
                type_descriptions.join("; ")
            ));
        }

        // Add key facts
        if !facts.is_empty() {
            let fact_sample: Vec<&String> = facts.iter().take(3).collect();
            parts.push(format!(
                "Key facts: {}",
                fact_sample
                    .iter()
                    .map(|f| f.as_str())
                    .collect::<Vec<_>>()
                    .join(". ")
            ));
        }

        CommunitySummaryResult {
            community_id: community.id.as_str().to_string(),
            summary: parts.join(" "),
            cost_usd: 0.0,
        }
    }

    /// Summarize all communities that don't have summaries yet (or are dirty).
    pub async fn summarize_all(
        &self,
        group_id: Option<&str>,
    ) -> Result<Vec<CommunitySummaryResult>, SummarizerError> {
        let communities = self
            .driver
            .list_communities(group_id)
            .await
            .map_err(|e| SummarizerError::DriverError(e.to_string()))?;

        let mut results = Vec::new();

        for community in &communities {
            if community.summary.is_empty() || community.is_dirty {
                let result = self.summarize_community(community).await?;

                // Update community with new summary
                let mut updated = community.clone();
                updated.summary = result.summary.clone();
                updated.is_dirty = false;

                self.driver
                    .store_community(&updated)
                    .await
                    .map_err(|e| SummarizerError::DriverError(e.to_string()))?;

                results.push(result);
            }
        }

        Ok(results)
    }
}

impl CommunitySummaryResult {
    fn with_community_id(mut self, id: String) -> Self {
        self.community_id = id;
        self
    }
}

/// Errors from community summarization.
#[derive(Debug, thiserror::Error)]
pub enum SummarizerError {
    #[error("Driver error: {0}")]
    DriverError(String),

    #[error("LLM error: {0}")]
    LlmError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::memory::MemoryDriver;
    use crate::graph::{Entity, Relationship};

    #[tokio::test]
    async fn test_rule_based_summary() {
        let driver = Arc::new(MemoryDriver::new());

        let alice = Entity::new("Alice", "Person").with_summary("A researcher");
        let bob = Entity::new("Bob", "Person").with_summary("A founder");
        let anthropic = Entity::new("Anthropic", "Organization");

        driver.store_entity(&alice).await.unwrap();
        driver.store_entity(&bob).await.unwrap();
        driver.store_entity(&anthropic).await.unwrap();

        let rel = Relationship::new(
            alice.id.clone(), anthropic.id.clone(),
            "WORKS_AT", "Alice works at Anthropic",
        );
        driver.store_relationship(&rel).await.unwrap();

        let mut community = Community::new("test-community", 0);
        community.add_member(alice.id.clone());
        community.add_member(bob.id.clone());
        community.add_member(anthropic.id.clone());

        let summarizer = CommunitySummarizer::new(driver, None);
        let result = summarizer.summarize_community(&community).await.unwrap();

        assert!(!result.summary.is_empty(), "Summary should not be empty");
        assert!(result.summary.contains("3 entities"), "Summary should mention entity count");
        assert_eq!(result.cost_usd, 0.0, "Rule-based should be free");
    }

    #[tokio::test]
    async fn test_empty_community_summary() {
        let driver = Arc::new(MemoryDriver::new());
        let community = Community::new("empty", 0);

        let summarizer = CommunitySummarizer::new(driver, None);
        let result = summarizer.summarize_community(&community).await.unwrap();

        assert!(result.summary.is_empty());
    }
}
