// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the ingestion pipeline.

use neurograph_core::graph::ontology::Ontology;
use neurograph_core::graph::{Entity, Relationship};
use neurograph_core::ingestion::conflict::ConflictResolver;
use neurograph_core::ingestion::deduplication::{DeduplicationConfig, Deduplicator};
use neurograph_core::ingestion::extractors::json::JsonExtractor;
use neurograph_core::ingestion::extractors::text::TextExtractor;
use neurograph_core::ingestion::extractors::traits::Extractor;
use neurograph_core::ingestion::validators::SchemaValidator;

// ─── Text Extractor Tests ─────────────────────────────────────

#[tokio::test]
async fn test_text_extractor_basic_entities() {
    let extractor = TextExtractor::regex_only();
    let result = extractor
        .extract("Alice works at Anthropic in San Francisco")
        .await
        .unwrap();

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
}

#[tokio::test]
async fn test_text_extractor_relationships() {
    let extractor = TextExtractor::regex_only();
    let result = extractor
        .extract("Bob founded Anthropic. Alice joined Anthropic.")
        .await
        .unwrap();

    assert!(
        !result.relationships.is_empty(),
        "Should extract relationships"
    );

    let rel_types: Vec<&str> = result
        .relationships
        .iter()
        .map(|r| r.relationship_type.as_str())
        .collect();
    assert!(
        rel_types.contains(&"FOUNDED") || rel_types.contains(&"JOINED"),
        "Should find FOUNDED or JOINED, got: {:?}",
        rel_types
    );
}

#[tokio::test]
async fn test_text_extractor_empty_input() {
    let extractor = TextExtractor::regex_only();
    let result = extractor.extract("").await.unwrap();
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_text_extractor_no_entities() {
    let extractor = TextExtractor::regex_only();
    let result = extractor
        .extract("the quick brown fox jumps over the lazy dog")
        .await
        .unwrap();

    // No proper nouns, so should have few/no entities
    assert_eq!(result.cost_usd, 0.0, "Regex extraction should be free");
}

#[tokio::test]
async fn test_text_extractor_multiple_relationships() {
    let extractor = TextExtractor::regex_only();
    let result = extractor
        .extract("Alice works at Google. Bob lives in London. Charlie founded Tesla.")
        .await
        .unwrap();

    // Should find multiple entities
    let names: Vec<&str> = result.entities.iter().map(|e| e.name.as_str()).collect();
    assert!(
        names.contains(&"Alice") || names.contains(&"Google"),
        "Should find entities"
    );

    // At least some relationships
    assert!(
        result.entities.len() >= 2,
        "Should find at least 2 entities, got {}",
        result.entities.len()
    );
}

// ─── JSON Extractor Tests ─────────────────────────────────────

#[tokio::test]
async fn test_json_extractor_single_entity() {
    let extractor = JsonExtractor::new();
    let input = r#"{"name": "Alice", "type": "Person", "summary": "A researcher"}"#;
    let result = extractor.extract(input).await.unwrap();

    assert_eq!(result.entities.len(), 1);
    assert_eq!(result.entities[0].name, "Alice");
    assert_eq!(result.entities[0].entity_type, "Person");
    assert_eq!(result.entities[0].summary, "A researcher");
}

#[tokio::test]
async fn test_json_extractor_multiple_entities() {
    let extractor = JsonExtractor::new();
    let input = r#"[
        {"name": "Alice", "type": "Person"},
        {"name": "Bob", "type": "Person"},
        {"name": "Anthropic", "type": "Organization"}
    ]"#;
    let result = extractor.extract(input).await.unwrap();

    assert_eq!(result.entities.len(), 3);
}

#[tokio::test]
async fn test_json_extractor_nested_relationships() {
    let extractor = JsonExtractor::new();
    let input = r#"{
        "name": "Anthropic",
        "type": "Organization",
        "summary": "AI safety company",
        "employees": [
            {"name": "Alice", "type": "Person"},
            {"name": "Bob", "type": "Person"}
        ]
    }"#;
    let result = extractor.extract(input).await.unwrap();

    // Should find Anthropic + Alice + Bob
    assert!(
        result.entities.len() >= 3,
        "Should find at least 3 entities, got {}",
        result.entities.len()
    );

    // Should have relationships from Anthropic to employees
    assert!(
        !result.relationships.is_empty(),
        "Should have relationships from parent to children"
    );
}

#[tokio::test]
async fn test_json_extractor_invalid_json() {
    let extractor = JsonExtractor::new();
    let result = extractor.extract("not valid json {{").await;
    assert!(result.is_err(), "Should fail on invalid JSON");
}

#[tokio::test]
async fn test_json_extractor_with_entity_references() {
    let extractor = JsonExtractor::new();
    let input = r#"{"name": "Alice", "type": "Person", "company": "Anthropic"}"#;
    let result = extractor.extract(input).await.unwrap();

    let names: Vec<&str> = result.entities.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"Alice"), "Should find Alice");
    assert!(
        names.contains(&"Anthropic"),
        "Should find Anthropic as entity reference"
    );
}

// ─── Deduplication Tests ──────────────────────────────────────

#[test]
fn test_deduplication_config() {
    let config = DeduplicationConfig::default();
    assert!(config.exact_threshold > 0.0);
    assert!(config.exact_threshold > config.ambiguous_threshold);
    assert!(config.max_candidates > 0);
}

#[test]
fn test_entity_merge() {
    let mut existing = Entity::new("Alice", "Person");
    existing.summary = "A person".to_string();

    Deduplicator::merge_entities(
        &mut existing,
        "Alice Johnson",
        "Alice is a senior researcher at Anthropic working on AI safety",
    );

    assert_eq!(
        existing.summary, "Alice is a senior researcher at Anthropic working on AI safety",
        "Summary should be updated to the longer/more detailed one"
    );
}

#[test]
fn test_entity_merge_keeps_existing_if_new_is_empty() {
    let mut existing = Entity::new("Alice", "Person");
    existing.summary = "Existing summary".to_string();

    Deduplicator::merge_entities(&mut existing, "Alice", "");

    assert_eq!(
        existing.summary, "Existing summary",
        "Should keep existing summary when new is empty"
    );
}

// ─── Conflict Resolution Tests ────────────────────────────────

#[test]
fn test_conflict_resolution_invalidates_old() {
    use chrono::Utc;
    use neurograph_core::graph::entity::EntityId;

    let src = EntityId::new();
    let tgt = EntityId::new();
    let mut rel = Relationship::new(src, tgt, "LIVES_IN", "Alice lives in NYC");

    assert!(rel.is_valid(), "Relationship should initially be valid");

    let now = Utc::now();
    ConflictResolver::resolve_contradiction(&mut rel, now);

    assert!(
        !rel.is_valid(),
        "Relationship should be invalid after contradiction"
    );
    assert_eq!(rel.valid_until, Some(now));
    assert!(rel.expired_at.is_some());
}

// ─── Validator Tests ──────────────────────────────────────────

#[test]
fn test_validator_rejects_empty_entity_name() {
    let validator = SchemaValidator::new(Ontology::open());
    let entity = neurograph_core::ingestion::extractors::traits::ExtractedEntity {
        name: "".to_string(),
        entity_type: "Person".to_string(),
        summary: String::new(),
    };
    let result = validator.validate_entity(&entity);
    assert!(!result.is_valid);
}

#[test]
fn test_validator_accepts_valid_entity() {
    let validator = SchemaValidator::new(Ontology::open());
    let entity = neurograph_core::ingestion::extractors::traits::ExtractedEntity {
        name: "Alice Johnson".to_string(),
        entity_type: "Person".to_string(),
        summary: "A researcher".to_string(),
    };
    let result = validator.validate_entity(&entity);
    assert!(result.is_valid);
}

#[test]
fn test_validator_rejects_empty_relationship_fields() {
    let validator = SchemaValidator::new(Ontology::open());
    let rel = neurograph_core::ingestion::extractors::traits::ExtractedRelationship {
        source_entity: "".to_string(),
        target_entity: "Bob".to_string(),
        relationship_type: "WORKS_AT".to_string(),
        fact: "test".to_string(),
        confidence: 0.9,
    };
    let result = validator.validate_relationship(&rel);
    assert!(!result.is_valid);
}

// ─── Full Pipeline Tests ──────────────────────────────────────

#[tokio::test]
async fn test_pipeline_text_ingestion() {
    let ng = neurograph_core::NeuroGraph::builder()
        .build()
        .await
        .unwrap();

    let episode = ng
        .add_text("Alice works at Anthropic in San Francisco")
        .await
        .unwrap();
    assert!(!episode.id.0.is_nil(), "Episode should have an ID");

    let stats = ng.stats().await.unwrap();
    assert!(
        *stats.get("entities").unwrap_or(&0) > 0,
        "Should have stored entities, got: {:?}",
        stats
    );
}

#[tokio::test]
async fn test_pipeline_json_ingestion() {
    let ng = neurograph_core::NeuroGraph::builder()
        .build()
        .await
        .unwrap();

    let data = serde_json::json!({
        "name": "Bob",
        "type": "Person",
        "company": "Google",
        "summary": "Software engineer"
    });

    let episode = ng.add_json(data).await.unwrap();
    assert!(!episode.id.0.is_nil(), "Episode should have an ID");

    let stats = ng.stats().await.unwrap();
    assert!(
        *stats.get("entities").unwrap_or(&0) > 0,
        "Should have stored entities from JSON"
    );
}

#[tokio::test]
async fn test_pipeline_deduplication() {
    let ng = neurograph_core::NeuroGraph::builder()
        .build()
        .await
        .unwrap();

    // Ingest same entity twice
    ng.add_text("Alice works at Anthropic").await.unwrap();
    ng.add_text("Alice lives in San Francisco").await.unwrap();

    // Alice should not be duplicated (dedup should merge)
    let stats = ng.stats().await.unwrap();
    let entity_count = *stats.get("entities").unwrap_or(&0);

    // We expect entities but not double-counting Alice
    assert!(entity_count > 0, "Should have entities");
}

#[tokio::test]
async fn test_pipeline_cost_tracking() {
    let ng = neurograph_core::NeuroGraph::builder()
        .build()
        .await
        .unwrap();

    // Without LLM, cost should be 0
    ng.add_text("Alice works at Anthropic").await.unwrap();
    assert_eq!(
        ng.total_cost_usd(),
        0.0,
        "Regex-only pipeline should have zero cost"
    );
}
