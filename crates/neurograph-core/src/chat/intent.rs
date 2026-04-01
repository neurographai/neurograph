// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Intent classification for the chat agent.
//!
//! Two-stage classification:
//! 1. **Fast path**: Regex pattern matching (instant, free) — handles ~70% of queries
//! 2. **LLM fallback**: Structured JSON output via fast model — handles ambiguous queries

use serde::{Deserialize, Serialize};

/// The 11 intent types for user messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatIntent {
    /// Explain a concept, entity, or relationship.
    Explain,
    /// Explore connections from a node.
    Explore,
    /// Compare how something changed over time.
    TemporalCompare,
    /// View the graph at a specific past point.
    TimeTravel,
    /// Find conflicting facts.
    FindContradictions,
    /// Summarize a topic, community, or paper.
    Summarize,
    /// Search for entities, papers, or topics.
    Search,
    /// Trace relationship chains between two entities.
    TraceRelationship,
    /// Discover themes or clusters in the graph.
    DiscoverThemes,
    /// Filter/focus the graph view by criteria.
    FilterGraph,
    /// General conversation or fallback.
    General,
}

impl std::fmt::Display for ChatIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatIntent::Explain => write!(f, "explain"),
            ChatIntent::Explore => write!(f, "explore"),
            ChatIntent::TemporalCompare => write!(f, "temporal_compare"),
            ChatIntent::TimeTravel => write!(f, "time_travel"),
            ChatIntent::FindContradictions => write!(f, "find_contradictions"),
            ChatIntent::Summarize => write!(f, "summarize"),
            ChatIntent::Search => write!(f, "search"),
            ChatIntent::TraceRelationship => write!(f, "trace_relationship"),
            ChatIntent::DiscoverThemes => write!(f, "discover_themes"),
            ChatIntent::FilterGraph => write!(f, "filter_graph"),
            ChatIntent::General => write!(f, "general"),
        }
    }
}

impl ChatIntent {
    /// Readable label for UI display.
    pub fn display_label(&self) -> &'static str {
        match self {
            ChatIntent::Explain => "Explaining",
            ChatIntent::Explore => "Exploring",
            ChatIntent::TemporalCompare => "Comparing Timeline",
            ChatIntent::TimeTravel => "Time Travelling",
            ChatIntent::FindContradictions => "Finding Contradictions",
            ChatIntent::Summarize => "Summarizing",
            ChatIntent::Search => "Searching",
            ChatIntent::TraceRelationship => "Tracing Relationships",
            ChatIntent::DiscoverThemes => "Discovering Themes",
            ChatIntent::FilterGraph => "Filtering Graph",
            ChatIntent::General => "Thinking",
        }
    }
}

/// Result of intent classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedIntent {
    pub intent: ChatIntent,
    pub confidence: f32,
    pub method: ClassificationMethod,
    /// Extracted entities/keywords from the query.
    pub extracted_entities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationMethod {
    Regex,
    Llm,
}

/// Intent classifier with fast-path regex + LLM fallback.
pub struct IntentClassifier;

impl IntentClassifier {
    pub fn new() -> Self {
        Self
    }

    /// Classify intent from user message (fast path only — no LLM needed for most queries).
    pub fn classify(&self, message: &str) -> ClassifiedIntent {
        let lower = message.to_lowercase();

        // ── Fast path: regex patterns ──────────────────────────────
        let patterns: &[(&[&str], ChatIntent, f32)] = &[
            // Explain
            (
                &["what is", "what are", "explain", "define", "describe", "tell me about", "how does", "how do"],
                ChatIntent::Explain,
                0.85,
            ),
            // Explore
            (
                &["connected to", "related to", "neighbors of", "links from", "expand", "show connections"],
                ChatIntent::Explore,
                0.90,
            ),
            // Temporal Compare
            (
                &["changed", "evolved", "differ", "before and after", "over time", "progression", "trend"],
                ChatIntent::TemporalCompare,
                0.85,
            ),
            // Time Travel
            (
                &["as of", "at time", "back in", "in 2", "snapshot", "point in time", "version from"],
                ChatIntent::TimeTravel,
                0.90,
            ),
            // Contradictions
            (
                &["contradict", "conflict", "inconsisten", "disagree", "opposing", "wrong"],
                ChatIntent::FindContradictions,
                0.90,
            ),
            // Summarize
            (
                &["summarize", "summary", "overview", "recap", "tldr", "key points", "main ideas"],
                ChatIntent::Summarize,
                0.90,
            ),
            // Search
            (
                &["find", "search", "look up", "locate", "where is", "which paper", "who", "list all"],
                ChatIntent::Search,
                0.80,
            ),
            // Trace Relationships
            (
                &["path between", "connection between", "how is .* related to", "trace", "link between", "chain from"],
                ChatIntent::TraceRelationship,
                0.90,
            ),
            // Discover Themes
            (
                &["themes", "clusters", "communities", "topics", "group", "categories", "patterns"],
                ChatIntent::DiscoverThemes,
                0.85,
            ),
            // Filter Graph
            (
                &["filter", "show only", "hide", "focus on", "narrow", "just show", "remove"],
                ChatIntent::FilterGraph,
                0.90,
            ),
        ];

        for (keywords, intent, confidence) in patterns {
            for keyword in *keywords {
                if lower.contains(keyword) {
                    return ClassifiedIntent {
                        intent: *intent,
                        confidence: *confidence,
                        method: ClassificationMethod::Regex,
                        extracted_entities: extract_entities_from_query(&lower),
                    };
                }
            }
        }

        // Question marks with no other signal → Explain
        if lower.contains('?') {
            return ClassifiedIntent {
                intent: ChatIntent::Explain,
                confidence: 0.60,
                method: ClassificationMethod::Regex,
                extracted_entities: extract_entities_from_query(&lower),
            };
        }

        // Default: General
        ClassifiedIntent {
            intent: ChatIntent::General,
            confidence: 0.50,
            method: ClassificationMethod::Regex,
            extracted_entities: extract_entities_from_query(&lower),
        }
    }

    /// LLM-based classification prompt (for when the agent has an LLM router).
    pub fn llm_classification_prompt(message: &str) -> String {
        format!(
            r#"Classify the user's intent into exactly ONE of these categories:
- explain: asking about what something is
- explore: wanting to see connections/neighbors
- temporal_compare: comparing changes over time
- time_travel: wanting to see data at a specific past point
- find_contradictions: looking for conflicting information
- summarize: wants a summary of a topic
- search: looking for specific entities or papers
- trace_relationship: wants to trace paths between entities
- discover_themes: wants to see clusters/themes/communities
- filter_graph: wants to filter the graph view
- general: general chat or unclear intent

User message: "{}"

Respond with JSON only: {{"intent": "...", "confidence": 0.0-1.0, "entities": ["..."]}}
"#,
            message
        )
    }
}

impl Default for IntentClassifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract potential entity names from a query (simple heuristic).
fn extract_entities_from_query(query: &str) -> Vec<String> {
    let stop_words = [
        "what", "is", "the", "a", "an", "of", "in", "to", "for", "and", "or", "how", "does",
        "do", "are", "was", "were", "been", "be", "can", "could", "would", "should", "will",
        "did", "has", "have", "had", "about", "tell", "me", "explain", "describe", "show",
        "find", "search", "between", "from", "with", "by", "at", "on", "it", "this", "that",
        "these", "those", "all", "related", "connected", "links", "only", "just", "my",
    ];
    let stop_set: std::collections::HashSet<&str> = stop_words.iter().copied().collect();

    query
        .split_whitespace()
        .filter(|w| {
            let clean = w.trim_matches(|c: char| !c.is_alphanumeric());
            clean.len() > 2 && !stop_set.contains(clean)
        })
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explain_intent() {
        let classifier = IntentClassifier::new();
        let result = classifier.classify("What is a transformer architecture?");
        assert_eq!(result.intent, ChatIntent::Explain);
        assert!(result.confidence >= 0.8);
    }

    #[test]
    fn test_summarize_intent() {
        let classifier = IntentClassifier::new();
        let result = classifier.classify("Summarize the attention is all you need paper");
        assert_eq!(result.intent, ChatIntent::Summarize);
    }

    #[test]
    fn test_search_intent() {
        let classifier = IntentClassifier::new();
        let result = classifier.classify("Find all papers about graph neural networks");
        assert_eq!(result.intent, ChatIntent::Search);
    }

    #[test]
    fn test_temporal_intent() {
        let classifier = IntentClassifier::new();
        let result = classifier.classify("How has the concept of attention changed over time?");
        assert_eq!(result.intent, ChatIntent::TemporalCompare);
    }

    #[test]
    fn test_explore_intent() {
        let classifier = IntentClassifier::new();
        let result = classifier.classify("Show connections to BERT");
        assert_eq!(result.intent, ChatIntent::Explore);
    }

    #[test]
    fn test_general_fallback() {
        let classifier = IntentClassifier::new();
        let result = classifier.classify("Hello there");
        assert_eq!(result.intent, ChatIntent::General);
    }
}
