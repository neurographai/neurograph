// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Text-based entity and relationship extraction.
//!
//! Two modes:
//! 1. **LLM mode** (default): Uses structured JSON output from an LLM to extract
//!    entities and relationships. Influenced by Graphiti's `add_episode()` approach
//!    and GraphRAG's extraction prompts.
//! 2. **Regex/NLP mode** (fallback): Rule-based extraction using regex patterns
//!    for Named Entity Recognition when no LLM is available.
//!
//! The LLM mode produces higher quality but costs tokens.
//! The regex mode is free but catches fewer entities.

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::llm::traits::{complete_structured, CompletionRequest, LlmClient};

use super::traits::{
    ExtractedEntity, ExtractedRelationship, ExtractionError, ExtractionResult, Extractor,
};

/// System prompt for LLM-based entity/relationship extraction.
///
/// Influenced by Graphiti's extraction prompts (prompts/extract_nodes.py)
/// and GraphRAG's entity extraction (index/operations/extract_entities.py).
/// Uses JSON mode for reliable structured output.
const EXTRACTION_SYSTEM_PROMPT: &str = r#"You are an expert knowledge graph engineer. Extract all entities and relationships from the given text.

Rules:
1. Entity names should be canonical (e.g., "Alice Johnson" not "she", "Anthropic" not "the company")
2. Entity types should be one of: Person, Organization, Location, Event, Concept, Product, Date, Other
3. Relationship types should be uppercase with underscores (e.g., WORKS_AT, LIVES_IN, FOUNDED)
4. Facts should be complete sentences describing the relationship
5. Include ALL entities and relationships, even implicit ones
6. Confidence should reflect how explicit the relationship is (1.0 = stated directly, 0.5 = implied)

Respond ONLY with valid JSON in this exact format:
{
  "entities": [
    {"name": "Entity Name", "entity_type": "Person", "summary": "Brief description"}
  ],
  "relationships": [
    {
      "source_entity": "Entity A",
      "target_entity": "Entity B",
      "relationship_type": "WORKS_AT",
      "fact": "Entity A works at Entity B",
      "confidence": 0.95
    }
  ]
}"#;

/// LLM extraction response structure (for JSON deserialization).
#[derive(Debug, Deserialize, Serialize)]
struct LlmExtractionResponse {
    entities: Vec<ExtractedEntity>,
    relationships: Vec<ExtractedRelationship>,
}

/// Text extractor that uses an LLM for entity/relationship extraction,
/// with a regex-based fallback when no LLM is available.
pub struct TextExtractor {
    /// Optional LLM client for high-quality extraction.
    llm: Option<Arc<dyn LlmClient>>,
}

impl TextExtractor {
    /// Create a text extractor with LLM support.
    pub fn with_llm(llm: Arc<dyn LlmClient>) -> Self {
        Self { llm: Some(llm) }
    }

    /// Create a text extractor without LLM (regex-only fallback).
    pub fn regex_only() -> Self {
        Self { llm: None }
    }

    /// LLM-based extraction: sends text to LLM with extraction prompt.
    async fn extract_with_llm(
        &self,
        input: &str,
        llm: &dyn LlmClient,
    ) -> Result<ExtractionResult, ExtractionError> {
        let start = Instant::now();

        let request = CompletionRequest::new(format!(
            "Extract all entities and relationships from this text:\n\n{}",
            input
        ))
        .with_system(EXTRACTION_SYSTEM_PROMPT)
        .with_json_mode()
        .with_temperature(0.0);

        let (response, usage) = complete_structured::<LlmExtractionResponse>(llm, request)
            .await
            .map_err(|e| ExtractionError::LlmError(e.to_string()))?;

        Ok(ExtractionResult {
            entities: response.entities,
            relationships: response.relationships,
            cost_usd: usage.cost_usd,
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Regex-based extraction: uses pattern matching for Named Entity Recognition.
    ///
    /// This is a best-effort fallback. It captures:
    /// - Proper nouns (capitalized words not at sentence start)
    /// - Common patterns like "X works at Y", "X lives in Y"
    /// - Dates and locations (basic patterns)
    fn extract_with_regex(&self, input: &str) -> ExtractionResult {
        let start = Instant::now();
        let mut entities = Vec::new();
        let mut relationships = Vec::new();
        let mut seen_entities = std::collections::HashSet::new();

        // --- Entity extraction via capitalized word sequences ---
        // Matches sequences of capitalized words (proper nouns)
        // e.g., "Alice Johnson", "San Francisco", "Anthropic"
        let words: Vec<&str> = input.split_whitespace().collect();
        let mut i = 0;
        while i < words.len() {
            let word = words[i].trim_matches(|c: char| !c.is_alphanumeric());
            if !word.is_empty() && word.chars().next().is_some_and(|c| c.is_uppercase()) {
                // Check if this is a sentence start (skip common sentence-starting words)
                let is_sentence_start = i == 0
                    || (i > 0 && words[i - 1].ends_with('.'))
                    || (i > 0 && words[i - 1].ends_with('!'))
                    || (i > 0 && words[i - 1].ends_with('?'));

                // Collect consecutive capitalized words
                let mut name_parts = vec![word.to_string()];
                let mut j = i + 1;
                while j < words.len() {
                    let next = words[j].trim_matches(|c: char| !c.is_alphanumeric());
                    if !next.is_empty() && next.chars().next().is_some_and(|c| c.is_uppercase()) {
                        name_parts.push(next.to_string());
                        j += 1;
                    } else {
                        break;
                    }
                }

                let name = name_parts.join(" ");

                // Skip common English words that happen to be capitalized
                let skip_words = [
                    "The", "A", "An", "In", "On", "At", "To", "For", "And", "Or", "But", "Is",
                    "Was", "Are", "Were", "Has", "Had", "Have", "Be", "It", "He", "She", "They",
                    "We", "This", "That", "These", "Those", "My", "His", "Her", "Our", "Their",
                    "Its", "I", "If", "So", "As", "Of", "By", "From", "With", "Not", "No", "Yes",
                    "Do", "Did", "Does", "Will", "Would", "Could", "Should", "May", "Can", "Then",
                    "When", "Where", "How", "What", "Who", "Which", "After", "Before", "Since",
                    "While",
                ];

                let should_skip = is_sentence_start && name_parts.len() == 1
                    || skip_words.contains(&name.as_str())
                    || name.len() < 2;

                if !should_skip && !seen_entities.contains(&name) {
                    let entity_type = Self::infer_entity_type(&name, input);
                    entities.push(ExtractedEntity {
                        name: name.clone(),
                        entity_type,
                        summary: String::new(),
                    });
                    seen_entities.insert(name);
                }

                i = j;
            } else {
                i += 1;
            }
        }

        // --- Relationship extraction via patterns ---
        let patterns: &[(&str, &str)] = &[
            (" works at ", "WORKS_AT"),
            (" worked at ", "WORKED_AT"),
            (" works for ", "WORKS_FOR"),
            (" founded ", "FOUNDED"),
            (" co-founded ", "CO_FOUNDED"),
            (" lives in ", "LIVES_IN"),
            (" lived in ", "LIVED_IN"),
            (" moved to ", "MOVED_TO"),
            (" moved from ", "MOVED_FROM"),
            (" joined ", "JOINED"),
            (" left ", "LEFT"),
            (" started ", "STARTED"),
            (" married ", "MARRIED"),
            (" is the CEO of ", "CEO_OF"),
            (" is a ", "IS_A"),
            (" is an ", "IS_AN"),
            (" belongs to ", "BELONGS_TO"),
            (" located in ", "LOCATED_IN"),
            (" acquired ", "ACQUIRED"),
            (" partnered with ", "PARTNERED_WITH"),
            (" invested in ", "INVESTED_IN"),
            (" created ", "CREATED"),
            (" built ", "BUILT"),
            (" manages ", "MANAGES"),
            (" reports to ", "REPORTS_TO"),
            (" knows ", "KNOWS"),
        ];

        let input_lower = input.to_lowercase();
        for (pattern, rel_type) in patterns {
            if let Some(pos) = input_lower.find(pattern) {
                // Find entity before pattern
                let before = &input[..pos];
                let after_start = pos + pattern.len();
                if after_start >= input.len() {
                    continue;
                }
                let after = &input[after_start..];

                // Extract last capitalized sequence before pattern as source
                let source = Self::extract_last_proper_noun(before);
                // Extract first capitalized sequence after pattern as target
                let target = Self::extract_first_proper_noun(after);

                if let (Some(src), Some(tgt)) = (source, target) {
                    // Ensure source and target entities are in the entities list
                    for name in [&src, &tgt] {
                        if !seen_entities.contains(name) {
                            let entity_type = Self::infer_entity_type(name, input);
                            entities.push(ExtractedEntity {
                                name: name.clone(),
                                entity_type,
                                summary: String::new(),
                            });
                            seen_entities.insert(name.clone());
                        }
                    }

                    let fact = format!("{}{}{}", src, pattern, tgt);
                    relationships.push(ExtractedRelationship {
                        source_entity: src,
                        target_entity: tgt,
                        relationship_type: rel_type.to_string(),
                        fact,
                        confidence: 0.7, // Regex extraction is lower confidence
                    });
                }
            }
        }

        ExtractionResult {
            entities,
            relationships,
            cost_usd: 0.0, // Regex is free
            latency_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// Infer entity type from name and surrounding context.
    fn infer_entity_type(name: &str, context: &str) -> String {
        let ctx_lower = context.to_lowercase();
        let name_lower = name.to_lowercase();

        // Location indicators
        let location_words = [
            "city", "country", "state", "town", "village", "island", "mountain", "river", "lake",
            "ocean", "street", "avenue",
        ];
        let known_locations = [
            "new york",
            "san francisco",
            "sf",
            "nyc",
            "london",
            "paris",
            "tokyo",
            "los angeles",
            "la",
            "chicago",
            "boston",
            "seattle",
            "berlin",
            "beijing",
            "shanghai",
            "mumbai",
            "delhi",
            "bangalore",
            "usa",
            "us",
            "uk",
            "india",
            "china",
            "japan",
            "france",
            "germany",
        ];
        if known_locations.iter().any(|loc| name_lower == *loc)
            || location_words.iter().any(|w| {
                ctx_lower.contains(&format!("{} {}", name_lower, w))
                    || ctx_lower.contains(&format!("{} is a {}", name_lower, w))
            })
            || ctx_lower.contains(&format!("in {}", name_lower))
            || ctx_lower.contains(&format!("to {}", name_lower))
            || ctx_lower.contains(&format!("from {}", name_lower))
        {
            return "Location".to_string();
        }

        // Organization indicators
        let org_suffixes = [
            "Inc",
            "Corp",
            "LLC",
            "Ltd",
            "Co",
            "Company",
            "Foundation",
            "University",
            "Institute",
            "Association",
            "Group",
            "Labs",
        ];
        if org_suffixes.iter().any(|s| name.ends_with(s))
            || ctx_lower.contains(&format!("works at {}", name_lower))
            || ctx_lower.contains(&format!("joined {}", name_lower))
            || ctx_lower.contains(&format!("founded {}", name_lower))
            || ctx_lower.contains(&format!("left {}", name_lower))
            || ctx_lower.contains(&format!("ceo of {}", name_lower))
        {
            return "Organization".to_string();
        }

        // Date patterns
        let months = [
            "january",
            "february",
            "march",
            "april",
            "may",
            "june",
            "july",
            "august",
            "september",
            "october",
            "november",
            "december",
        ];
        if months.iter().any(|m| name_lower.contains(m))
            || name
                .chars()
                .all(|c| c.is_ascii_digit() || c == '-' || c == '/')
        {
            return "Date".to_string();
        }

        // Default: check if context suggests person
        let person_indicators = [
            "works", "lives", "born", "married", "moved", "said", "joined", "founded", "ceo",
            "mr.", "mrs.", "dr.", "prof.",
        ];
        if person_indicators.iter().any(|w| {
            ctx_lower.contains(&format!("{} {}", name_lower, w))
                || ctx_lower.contains(&format!("{} {}", w, name_lower))
        }) {
            return "Person".to_string();
        }

        // Default to "Entity" — the safe choice
        "Entity".to_string()
    }

    /// Extract the last proper noun sequence from text.
    fn extract_last_proper_noun(text: &str) -> Option<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut end = words.len();
        let mut parts = Vec::new();

        // Common words / titles that are capitalized but not proper nouns
        let stop_words = [
            "The",
            "A",
            "An",
            "In",
            "On",
            "At",
            "To",
            "For",
            "And",
            "Or",
            "But",
            "Is",
            "Was",
            "Are",
            "Were",
            "Has",
            "Had",
            "Have",
            "Be",
            "It",
            "He",
            "She",
            "They",
            "We",
            "This",
            "That",
            "These",
            "Those",
            "My",
            "His",
            "Her",
            "Our",
            "Their",
            "Its",
            "I",
            "If",
            "So",
            "As",
            "Of",
            "By",
            "From",
            "With",
            "Not",
            "CEO",
            "CTO",
            "CFO",
            "COO",
            "VP",
            "SVP",
            "EVP",
            "President",
            "Director",
            "Manager",
            "Chairman",
        ];

        // Walk backwards to find capitalized sequence, stopping at common words
        while end > 0 {
            let word = words[end - 1].trim_matches(|c: char| !c.is_alphanumeric());
            if !word.is_empty()
                && word.chars().next().is_some_and(|c| c.is_uppercase())
                && !stop_words.contains(&word)
            {
                parts.push(word.to_string());
                end -= 1;
            } else if parts.is_empty() {
                end -= 1;
            } else {
                break;
            }
        }

        if parts.is_empty() {
            None
        } else {
            parts.reverse();
            Some(parts.join(" "))
        }
    }

    /// Extract the first proper noun sequence from text.
    fn extract_first_proper_noun(text: &str) -> Option<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut parts = Vec::new();

        for word in &words {
            let clean = word.trim_matches(|c: char| !c.is_alphanumeric());
            if !clean.is_empty() && clean.chars().next().is_some_and(|c| c.is_uppercase()) {
                parts.push(clean.to_string());
            } else if !parts.is_empty() {
                break;
            }
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" "))
        }
    }
}

#[async_trait]
impl Extractor for TextExtractor {
    fn name(&self) -> &str {
        if self.llm.is_some() {
            "text-llm"
        } else {
            "text-regex"
        }
    }

    async fn extract(&self, input: &str) -> Result<ExtractionResult, ExtractionError> {
        if input.is_empty() {
            return Ok(ExtractionResult::empty());
        }

        // Check input size (100KB max for single extraction)
        if input.len() > 100_000 {
            return Err(ExtractionError::InputTooLarge {
                size: input.len(),
                max: 100_000,
            });
        }

        // Use LLM if available, regex fallback otherwise
        if let Some(ref llm) = self.llm {
            match self.extract_with_llm(input, llm.as_ref()).await {
                Ok(result) => {
                    tracing::info!(
                        extractor = "text-llm",
                        entities = result.entities.len(),
                        relationships = result.relationships.len(),
                        cost_usd = result.cost_usd,
                        "LLM extraction complete"
                    );
                    Ok(result)
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "LLM extraction failed, falling back to regex"
                    );
                    Ok(self.extract_with_regex(input))
                }
            }
        } else {
            let result = self.extract_with_regex(input);
            tracing::info!(
                extractor = "text-regex",
                entities = result.entities.len(),
                relationships = result.relationships.len(),
                "Regex extraction complete"
            );
            Ok(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_regex_extraction_basic() {
        let extractor = TextExtractor::regex_only();

        let result = extractor
            .extract("Alice works at Anthropic in San Francisco")
            .await
            .unwrap();

        // Should find entities
        let names: Vec<&str> = result.entities.iter().map(|e| e.name.as_str()).collect();
        assert!(
            names.contains(&"Alice"),
            "Should find Alice, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Anthropic"),
            "Should find Anthropic, got: {:?}",
            names
        );
        assert!(
            names.contains(&"San Francisco"),
            "Should find San Francisco, got: {:?}",
            names
        );

        // Should find at least one relationship
        assert!(
            !result.relationships.is_empty(),
            "Should extract at least one relationship"
        );

        // Cost should be zero for regex
        assert_eq!(result.cost_usd, 0.0);
    }

    #[tokio::test]
    async fn test_regex_extraction_relationships() {
        let extractor = TextExtractor::regex_only();

        let result = extractor
            .extract("Bob founded Anthropic. Alice joined Anthropic.")
            .await
            .unwrap();

        let rel_types: Vec<&str> = result
            .relationships
            .iter()
            .map(|r| r.relationship_type.as_str())
            .collect();

        assert!(
            rel_types.contains(&"FOUNDED") || rel_types.contains(&"JOINED"),
            "Should find FOUNDED or JOINED relationship, got: {:?}",
            rel_types
        );
    }

    #[tokio::test]
    async fn test_empty_input() {
        let extractor = TextExtractor::regex_only();
        let result = extractor.extract("").await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_entity_type_inference() {
        assert_eq!(
            TextExtractor::infer_entity_type("San Francisco", "Alice lives in San Francisco"),
            "Location"
        );
        assert_eq!(
            TextExtractor::infer_entity_type("Anthropic", "Alice works at Anthropic"),
            "Organization"
        );
        assert_eq!(
            TextExtractor::infer_entity_type("Alice", "Alice works at Anthropic"),
            "Person"
        );
    }

    #[tokio::test]
    async fn test_proper_noun_extraction() {
        assert_eq!(
            TextExtractor::extract_first_proper_noun("works at Anthropic Labs today"),
            Some("Anthropic Labs".to_string())
        );
        assert_eq!(
            TextExtractor::extract_last_proper_noun("The CEO Alice Johnson works"),
            Some("Alice Johnson".to_string())
        );
    }
}
