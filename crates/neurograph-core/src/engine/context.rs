// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Context assembly: graph data → LLM prompt.
//!
//! Takes retrieved entities, relationships, and community summaries
//! and assembles them into a prompt that fits within a token budget.
//!
//! Influenced by GraphRAG's context_builder (structured_search/context_builder.py)
//! which carefully manages token budgets when building prompts.

use crate::graph::{Community, Entity, Relationship};

/// Assembled context ready for LLM prompt.
#[derive(Debug, Clone)]
pub struct AssembledContext {
    /// The context text to include in the LLM prompt.
    pub context_text: String,
    /// Number of entities included.
    pub entity_count: usize,
    /// Number of relationships included.
    pub relationship_count: usize,
    /// Number of community summaries included.
    pub community_count: usize,
    /// Estimated token count of the context.
    pub estimated_tokens: usize,
}

/// Builds context from graph data for LLM prompts.
pub struct ContextBuilder {
    /// Maximum tokens for the context section.
    max_context_tokens: usize,
}

impl ContextBuilder {
    /// Create a new context builder with a token budget.
    pub fn new(max_context_tokens: usize) -> Self {
        Self { max_context_tokens }
    }

    /// Assemble context from entities, relationships, and communities.
    ///
    /// Priority order:
    /// 1. Directly relevant entities (with summaries)
    /// 2. Relationships as facts
    /// 3. Community summaries (for global context)
    ///
    /// Stops adding content when the token budget is reached.
    pub fn build(
        &self,
        entities: &[Entity],
        relationships: &[Relationship],
        communities: &[Community],
        _query: &str,
    ) -> AssembledContext {
        let mut sections = Vec::new();
        let mut total_tokens = 0;
        let mut entity_count = 0;
        let mut rel_count = 0;
        let mut community_count = 0;

        // Section 1: Entity descriptions
        if !entities.is_empty() {
            let mut entity_lines = Vec::new();
            entity_lines.push("## Relevant Entities".to_string());

            for entity in entities {
                let line = if entity.summary.is_empty() {
                    format!("- **{}** ({})", entity.name, entity.entity_type)
                } else {
                    format!(
                        "- **{}** ({}): {}",
                        entity.name, entity.entity_type, entity.summary
                    )
                };

                let tokens = Self::estimate_tokens(&line);
                if total_tokens + tokens > self.max_context_tokens {
                    break;
                }

                entity_lines.push(line);
                total_tokens += tokens;
                entity_count += 1;
            }

            if entity_count > 0 {
                sections.push(entity_lines.join("\n"));
            }
        }

        // Section 2: Relationships as facts
        if !relationships.is_empty() {
            let mut rel_lines = Vec::new();
            rel_lines.push("\n## Known Facts".to_string());

            for rel in relationships {
                let validity = if rel.is_valid() {
                    "✓ current"
                } else {
                    "✗ historical"
                };

                let line = format!("- {} [{}]", rel.fact, validity);

                let tokens = Self::estimate_tokens(&line);
                if total_tokens + tokens > self.max_context_tokens {
                    break;
                }

                rel_lines.push(line);
                total_tokens += tokens;
                rel_count += 1;
            }

            if rel_count > 0 {
                sections.push(rel_lines.join("\n"));
            }
        }

        // Section 3: Community summaries
        if !communities.is_empty() {
            let mut comm_lines = Vec::new();
            comm_lines.push("\n## Community Context".to_string());

            for community in communities {
                if community.summary.is_empty() {
                    continue;
                }

                let line = format!("- **{}**: {}", community.name, community.summary);

                let tokens = Self::estimate_tokens(&line);
                if total_tokens + tokens > self.max_context_tokens {
                    break;
                }

                comm_lines.push(line);
                total_tokens += tokens;
                community_count += 1;
            }

            if community_count > 0 {
                sections.push(comm_lines.join("\n"));
            }
        }

        let context_text = if sections.is_empty() {
            "No relevant information found in the knowledge graph.".to_string()
        } else {
            sections.join("\n")
        };

        AssembledContext {
            context_text,
            entity_count,
            relationship_count: rel_count,
            community_count,
            estimated_tokens: total_tokens,
        }
    }

    /// Rough token estimation (≈ 4 chars per token for English).
    fn estimate_tokens(text: &str) -> usize {
        text.len().div_ceil(4)
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new(4000) // ~4k tokens default context budget
    }
}

/// System prompt for answer generation.
pub const ANSWER_SYSTEM_PROMPT: &str = r#"You are a helpful assistant that answers questions based on a knowledge graph.

Use the provided context to answer the user's question. Follow these rules:
1. Only use information from the context — do not make up facts
2. If the context doesn't contain enough information, say so clearly
3. Cite specific entities and facts from the context
4. Note if any facts are marked as historical (no longer current)
5. Be concise and direct"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_assembly() {
        let builder = ContextBuilder::new(4000);

        let entities = vec![
            Entity::new("Alice", "Person").with_summary("A researcher at Anthropic"),
            Entity::new("Anthropic", "Organization").with_summary("An AI safety company"),
        ];

        let rels = vec![Relationship::new(
            entities[0].id.clone(),
            entities[1].id.clone(),
            "WORKS_AT",
            "Alice works at Anthropic as a researcher",
        )];

        let ctx = builder.build(&entities, &rels, &[], "Where does Alice work?");

        assert!(ctx.context_text.contains("Alice"));
        assert!(ctx.context_text.contains("Anthropic"));
        assert!(ctx.context_text.contains("works at"));
        assert_eq!(ctx.entity_count, 2);
        assert_eq!(ctx.relationship_count, 1);
    }

    #[test]
    fn test_empty_context() {
        let builder = ContextBuilder::new(4000);
        let ctx = builder.build(&[], &[], &[], "test query");
        assert!(ctx.context_text.contains("No relevant information"));
    }

    #[test]
    fn test_token_budget() {
        let builder = ContextBuilder::new(10); // Very small budget

        let entities: Vec<Entity> = (0..100)
            .map(|i| {
                Entity::new(format!("Entity{}", i), "Test")
                    .with_summary("A very long summary that takes up lots of tokens")
            })
            .collect();

        let ctx = builder.build(&entities, &[], &[], "test");
        assert!(
            ctx.entity_count < 100,
            "Should have been truncated by budget"
        );
    }
}
