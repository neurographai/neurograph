// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the NeuroGraph public API.

use neurograph_core::{Entity, NeuroGraph, Relationship};

#[tokio::test]
async fn test_neurograph_builder_default() {
    let ng = NeuroGraph::builder().build().await.unwrap();
    assert_eq!(ng.config().name, "neurograph");
}

#[tokio::test]
async fn test_neurograph_builder_named() {
    let ng = NeuroGraph::builder()
        .name("my-project")
        .build()
        .await
        .unwrap();
    assert_eq!(ng.config().name, "my-project");
}

#[tokio::test]
async fn test_neurograph_builder_with_budget() {
    let ng = NeuroGraph::builder()
        .budget(5.0)
        .build()
        .await
        .unwrap();
    assert_eq!(ng.config().budget_usd, Some(5.0));
}

#[tokio::test]
async fn test_neurograph_add_text() {
    let ng = NeuroGraph::builder().build().await.unwrap();

    let episode = ng.add_text("Alice works at Anthropic").await.unwrap();
    assert_eq!(episode.content, "Alice works at Anthropic");
    assert_eq!(episode.source_type, neurograph_core::graph::episode::EpisodeType::Text);
}

#[tokio::test]
async fn test_neurograph_add_json() {
    let ng = NeuroGraph::builder().build().await.unwrap();

    let data = serde_json::json!({
        "name": "Alice",
        "company": "Anthropic"
    });

    let episode = ng.add_json(data).await.unwrap();
    assert!(episode.content.contains("Alice"));
    assert_eq!(episode.source_type, neurograph_core::graph::episode::EpisodeType::Json);
}

#[tokio::test]
async fn test_neurograph_entity_crud() {
    let ng = NeuroGraph::builder().build().await.unwrap();

    // Create and store entity
    let entity = Entity::new("Alice", "Person")
        .with_summary("A researcher at Anthropic");

    ng.store_entity(&entity).await.unwrap();

    // Retrieve
    let retrieved = ng.get_entity(&entity.id).await.unwrap();
    assert_eq!(retrieved.name, "Alice");
    assert_eq!(retrieved.summary, "A researcher at Anthropic");

    // Should have embedding now (auto-generated)
    assert!(retrieved.name_embedding.is_some());
}

#[tokio::test]
async fn test_neurograph_relationship_crud() {
    let ng = NeuroGraph::builder().build().await.unwrap();

    let alice = Entity::new("Alice", "Person");
    let anthropic = Entity::new("Anthropic", "Organization");

    ng.store_entity(&alice).await.unwrap();
    ng.store_entity(&anthropic).await.unwrap();

    let rel = Relationship::new(
        alice.id.clone(),
        anthropic.id.clone(),
        "WORKS_AT",
        "Alice works at Anthropic as a research scientist",
    );

    ng.store_relationship(&rel).await.unwrap();

    let rels = ng.get_relationships(&alice.id).await.unwrap();
    assert_eq!(rels.len(), 1);
    assert_eq!(rels[0].fact, "Alice works at Anthropic as a research scientist");
}

#[tokio::test]
async fn test_neurograph_search() {
    let ng = NeuroGraph::builder().build().await.unwrap();

    // Store several entities
    let entities = vec![
        Entity::new("Machine Learning", "Concept"),
        Entity::new("Deep Learning", "Concept"),
        Entity::new("Natural Language Processing", "Concept"),
        Entity::new("Cooking Recipes", "Concept"),
    ];

    for entity in &entities {
        ng.store_entity(entity).await.unwrap();
    }

    // Search - should find ML-related entities
    let results = ng.search_entities("machine learning", 10).await.unwrap();
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_neurograph_traversal() {
    let ng = NeuroGraph::builder().build().await.unwrap();

    let alice = Entity::new("Alice", "Person");
    let bob = Entity::new("Bob", "Person");
    let anthropic = Entity::new("Anthropic", "Organization");

    ng.store_entity(&alice).await.unwrap();
    ng.store_entity(&bob).await.unwrap();
    ng.store_entity(&anthropic).await.unwrap();

    ng.store_relationship(&Relationship::new(
        alice.id.clone(),
        anthropic.id.clone(),
        "WORKS_AT",
        "Alice works at Anthropic",
    ))
    .await
    .unwrap();

    ng.store_relationship(&Relationship::new(
        bob.id.clone(),
        anthropic.id.clone(),
        "WORKS_AT",
        "Bob works at Anthropic",
    ))
    .await
    .unwrap();

    // Traverse from Alice with depth 2 should find Bob via Anthropic
    let subgraph = ng.traverse(&alice.id, 2).await.unwrap();
    assert_eq!(subgraph.entities.len(), 3);
    assert_eq!(subgraph.relationships.len(), 2);
}

#[tokio::test]
async fn test_neurograph_stats() {
    let ng = NeuroGraph::builder().build().await.unwrap();

    ng.store_entity(&Entity::new("A", "T")).await.unwrap();
    ng.store_entity(&Entity::new("B", "T")).await.unwrap();

    let stats = ng.stats().await.unwrap();
    assert_eq!(stats["entities"], 2);
}

#[tokio::test]
async fn test_neurograph_clear() {
    let ng = NeuroGraph::builder().build().await.unwrap();

    ng.store_entity(&Entity::new("A", "T")).await.unwrap();
    let stats = ng.stats().await.unwrap();
    assert_eq!(stats["entities"], 1);

    ng.clear().await.unwrap();
    let stats = ng.stats().await.unwrap();
    assert_eq!(stats["entities"], 0);
}

#[tokio::test]
async fn test_neurograph_embedded_storage() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test_ng_db");

    let ng = NeuroGraph::builder()
        .embedded(path.to_str().unwrap())
        .build()
        .await
        .unwrap();

    let entity = Entity::new("Alice", "Person");
    ng.store_entity(&entity).await.unwrap();

    let stats = ng.stats().await.unwrap();
    assert_eq!(stats["entities"], 1);
}

#[tokio::test]
async fn test_neurograph_schema_tracking() {
    let ng = NeuroGraph::builder().build().await.unwrap();

    ng.store_entity(&Entity::new("Alice", "Person")).await.unwrap();
    ng.store_entity(&Entity::new("Bob", "Person")).await.unwrap();
    ng.store_entity(&Entity::new("Anthropic", "Organization")).await.unwrap();

    let schema = ng.schema();
    assert_eq!(schema.total_entities, 3);
    assert!(schema.known_entity_types.contains("Person"));
    assert!(schema.known_entity_types.contains("Organization"));
    assert_eq!(schema.entity_type_counts["Person"], 2);
}
