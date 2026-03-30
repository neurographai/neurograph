// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! JSON-based entity extraction.
//!
//! Extracts entities and relationships directly from structured JSON data
//! without needing an LLM. Each top-level key becomes an entity attribute,
//! and nested objects/arrays become relationships.
//!
//! Influenced by Graphiti's `EpisodeType.json` handling and
//! Mem0's structured memory ingestion.

use async_trait::async_trait;
use std::time::Instant;

use super::traits::{ExtractionError, ExtractionResult, ExtractedEntity, ExtractedRelationship, Extractor};

/// Extracts entities from structured JSON data.
///
/// Mapping rules:
/// - Objects with a "name" field become entities
/// - The "type" or "entity_type" field sets the entity type
/// - Nested objects become separate entities with relationships
/// - Arrays of objects create multiple entities
pub struct JsonExtractor;

impl JsonExtractor {
    /// Create a new JSON extractor.
    pub fn new() -> Self {
        Self
    }

    /// Recursively extract entities from a JSON value.
    fn extract_from_value(
        &self,
        value: &serde_json::Value,
        parent_name: Option<&str>,
        entities: &mut Vec<ExtractedEntity>,
        relationships: &mut Vec<ExtractedRelationship>,
    ) {
        match value {
            serde_json::Value::Object(map) => {
                // Try to extract an entity from this object
                let name = map
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let entity_type = map
                    .get("type")
                    .or_else(|| map.get("entity_type"))
                    .or_else(|| map.get("category"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Entity")
                    .to_string();

                let summary = map
                    .get("summary")
                    .or_else(|| map.get("description"))
                    .or_else(|| map.get("bio"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if let Some(ref entity_name) = name {
                    entities.push(ExtractedEntity {
                        name: entity_name.clone(),
                        entity_type: entity_type.clone(),
                        summary: summary.clone(),
                    });

                    // Create relationship to parent if exists
                    if let Some(parent) = parent_name {
                        relationships.push(ExtractedRelationship {
                            source_entity: parent.to_string(),
                            target_entity: entity_name.clone(),
                            relationship_type: "HAS_MEMBER".to_string(),
                            fact: format!("{} includes {}", parent, entity_name),
                            confidence: 1.0,
                        });
                    }

                    // Process nested fields for relationships
                    for (key, val) in map {
                        if key == "name" || key == "type" || key == "entity_type"
                            || key == "category" || key == "summary"
                            || key == "description" || key == "bio"
                        {
                            continue;
                        }

                        match val {
                            serde_json::Value::String(s) => {
                                // Check if the string value looks like an entity reference
                                let rel_type = key.to_uppercase().replace(' ', "_");
                                if Self::looks_like_entity_name(s) {
                                    // Create the referenced entity
                                    entities.push(ExtractedEntity {
                                        name: s.clone(),
                                        entity_type: Self::infer_type_from_key(key),
                                        summary: String::new(),
                                    });
                                    relationships.push(ExtractedRelationship {
                                        source_entity: entity_name.clone(),
                                        target_entity: s.clone(),
                                        relationship_type: rel_type,
                                        fact: format!("{} {} {}", entity_name, key, s),
                                        confidence: 1.0,
                                    });
                                }
                            }
                            serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                                self.extract_from_value(val, Some(entity_name), entities, relationships);
                            }
                            _ => {}
                        }
                    }
                } else {
                    // No name field — try each sub-value
                    for (_key, val) in map {
                        self.extract_from_value(val, parent_name, entities, relationships);
                    }
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    self.extract_from_value(item, parent_name, entities, relationships);
                }
            }
            _ => {} // Primitives are ignored at the top level
        }
    }

    /// Check if a string looks like an entity name (capitalized, no long sentences).
    fn looks_like_entity_name(s: &str) -> bool {
        !s.is_empty()
            && s.len() < 100
            && !s.contains('\n')
            && s.chars().next().is_some_and(|c| c.is_uppercase())
    }

    /// Infer entity type from the JSON key name.
    fn infer_type_from_key(key: &str) -> String {
        let key_lower = key.to_lowercase();
        if key_lower.contains("company") || key_lower.contains("org") || key_lower.contains("employer") {
            "Organization".to_string()
        } else if key_lower.contains("city") || key_lower.contains("location")
            || key_lower.contains("country") || key_lower.contains("place")
        {
            "Location".to_string()
        } else if key_lower.contains("person") || key_lower.contains("author")
            || key_lower.contains("user") || key_lower.contains("manager")
        {
            "Person".to_string()
        } else {
            "Entity".to_string()
        }
    }
}

impl Default for JsonExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Extractor for JsonExtractor {
    fn name(&self) -> &str {
        "json"
    }

    async fn extract(&self, input: &str) -> Result<ExtractionResult, ExtractionError> {
        let start = Instant::now();

        let value: serde_json::Value = serde_json::from_str(input)
            .map_err(|e| ExtractionError::ParseError(format!("Invalid JSON: {}", e)))?;

        let mut entities = Vec::new();
        let mut relationships = Vec::new();

        self.extract_from_value(&value, None, &mut entities, &mut relationships);

        // Deduplicate entities by name
        let mut seen = std::collections::HashSet::new();
        entities.retain(|e| seen.insert(e.name.clone()));

        Ok(ExtractionResult {
            entities,
            relationships,
            cost_usd: 0.0,
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_json_entity_extraction() {
        let extractor = JsonExtractor::new();

        let input = r#"{"name": "Bob", "type": "Person", "company": "Anthropic", "role": "CEO"}"#;
        let result = extractor.extract(input).await.unwrap();

        let names: Vec<&str> = result.entities.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"Bob"), "Should find Bob, got: {:?}", names);
        assert!(names.contains(&"Anthropic"), "Should find Anthropic, got: {:?}", names);
    }

    #[tokio::test]
    async fn test_json_array_extraction() {
        let extractor = JsonExtractor::new();

        let input = r#"[
            {"name": "Alice", "type": "Person"},
            {"name": "Bob", "type": "Person"}
        ]"#;
        let result = extractor.extract(input).await.unwrap();

        assert_eq!(result.entities.len(), 2);
    }

    #[tokio::test]
    async fn test_json_nested_relationships() {
        let extractor = JsonExtractor::new();

        let input = r#"{"name": "Anthropic", "type": "Organization", "summary": "AI safety company"}"#;
        let result = extractor.extract(input).await.unwrap();

        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.entities[0].entity_type, "Organization");
        assert_eq!(result.entities[0].summary, "AI safety company");
    }

    #[tokio::test]
    async fn test_invalid_json() {
        let extractor = JsonExtractor::new();
        let result = extractor.extract("not json").await;
        assert!(result.is_err());
    }
}
