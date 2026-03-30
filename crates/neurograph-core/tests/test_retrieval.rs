// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the retrieval engine.

use neurograph_core::drivers::memory::MemoryDriver;
use neurograph_core::drivers::traits::GraphDriver;
use neurograph_core::embedders::fastembed::HashEmbedder;
use neurograph_core::embedders::traits::Embedder;
use neurograph_core::engine::budget::QueryBudget;
use neurograph_core::engine::context::ContextBuilder;
use neurograph_core::engine::router::{QueryRouter, QueryType};
use neurograph_core::graph::{Entity, Relationship};
use neurograph_core::retrieval::hybrid::{HybridRetriever, RetrievalWeights};
use neurograph_core::retrieval::keyword::KeywordSearcher;
use neurograph_core::retrieval::recipes::SearchRecipes;
use neurograph_core::retrieval::semantic::SemanticSearcher;
use neurograph_core::retrieval::traversal::TraversalSearcher;

use std::sync::Arc;

/// Helper: create a test graph with entities and relationships.
async fn setup_test_graph() -> (Arc<MemoryDriver>, Arc<HashEmbedder>) {
    let driver = Arc::new(MemoryDriver::new());
    let embedder = Arc::new(HashEmbedder::default());

    // Create entities with embeddings
    let alice = Entity::new("Alice", "Person")
        .with_summary("A researcher at Anthropic")
        .with_group_id("test")
        .with_embedding(embedder.embed_one("Alice").await.unwrap());

    let bob = Entity::new("Bob", "Person")
        .with_summary("Founder of Anthropic")
        .with_group_id("test")
        .with_embedding(embedder.embed_one("Bob").await.unwrap());

    let anthropic = Entity::new("Anthropic", "Organization")
        .with_summary("An AI safety company founded in 2021")
        .with_group_id("test")
        .with_embedding(embedder.embed_one("Anthropic").await.unwrap());

    let sf = Entity::new("San Francisco", "Location")
        .with_summary("A city in California")
        .with_group_id("test")
        .with_embedding(embedder.embed_one("San Francisco").await.unwrap());

    driver.store_entity(&alice).await.unwrap();
    driver.store_entity(&bob).await.unwrap();
    driver.store_entity(&anthropic).await.unwrap();
    driver.store_entity(&sf).await.unwrap();

    // Create relationships
    let rel1 = Relationship::new(
        alice.id.clone(),
        anthropic.id.clone(),
        "WORKS_AT",
        "Alice works at Anthropic as a researcher",
    )
    .with_group_id("test");

    let rel2 = Relationship::new(
        bob.id.clone(),
        anthropic.id.clone(),
        "FOUNDED",
        "Bob founded Anthropic in 2021",
    )
    .with_group_id("test");

    let rel3 = Relationship::new(
        anthropic.id.clone(),
        sf.id.clone(),
        "LOCATED_IN",
        "Anthropic is located in San Francisco",
    )
    .with_group_id("test");

    driver.store_relationship(&rel1).await.unwrap();
    driver.store_relationship(&rel2).await.unwrap();
    driver.store_relationship(&rel3).await.unwrap();

    (driver, embedder)
}

// ─── Semantic Search Tests ────────────────────────────────────

#[tokio::test]
async fn test_semantic_search_by_text() {
    let (driver, embedder) = setup_test_graph().await;

    let results = SemanticSearcher::search("Alice", 5, None, embedder.as_ref(), driver.as_ref())
        .await
        .unwrap();

    assert!(!results.is_empty(), "Should find results for 'Alice'");
    assert_eq!(results[0].source, "semantic");
}

#[tokio::test]
async fn test_semantic_search_by_vector() {
    let (driver, embedder) = setup_test_graph().await;

    let embedding = embedder.embed_one("Anthropic").await.unwrap();
    let results = SemanticSearcher::search_by_vector(&embedding, 5, None, driver.as_ref())
        .await
        .unwrap();

    assert!(!results.is_empty(), "Should find results by vector");
}

// ─── Keyword Search Tests ─────────────────────────────────────

#[tokio::test]
async fn test_keyword_search() {
    let (driver, _embedder) = setup_test_graph().await;

    let results = KeywordSearcher::search("Alice", 5, None, driver.as_ref())
        .await
        .unwrap();

    assert!(!results.is_empty(), "Should find Alice by keyword");
    assert_eq!(results[0].source, "keyword");
}

#[tokio::test]
async fn test_keyword_search_no_results() {
    let (driver, _embedder) = setup_test_graph().await;

    let results = KeywordSearcher::search("nonexistent_entity_xyz_12345", 5, None, driver.as_ref())
        .await
        .unwrap();

    assert!(
        results.is_empty(),
        "Should find no results for nonexistent query"
    );
}

// ─── Traversal Search Tests ───────────────────────────────────

#[tokio::test]
async fn test_traversal_from_seed() {
    let (driver, embedder) = setup_test_graph().await;

    // First find Alice to get her ID
    let embedding = embedder.embed_one("Alice").await.unwrap();
    let search_results = driver
        .search_entities_by_vector(&embedding, 1, None)
        .await
        .unwrap();

    if let Some(alice_result) = search_results.first() {
        let results = TraversalSearcher::search(
            &[alice_result.entity.id.clone()],
            2,
            10,
            None,
            driver.as_ref(),
        )
        .await
        .unwrap();

        // Should find connected entities (Anthropic, possibly Bob, SF)
        assert!(
            !results.is_empty(),
            "Should find entities connected to Alice"
        );
        assert_eq!(results[0].source, "traversal");
    }
}

// ─── Hybrid Search Tests ──────────────────────────────────────

#[tokio::test]
async fn test_hybrid_search() {
    let (driver, embedder) = setup_test_graph().await;

    let retriever = HybridRetriever::new();
    let results = retriever
        .search(
            "Alice researcher",
            5,
            None,
            Some("test"),
            embedder.as_ref(),
            driver.as_ref(),
        )
        .await
        .unwrap();

    assert!(!results.is_empty(), "Should find results via hybrid search");
    // Results should have fused scores
    for r in &results {
        assert!(r.score > 0.0, "Fused score should be positive");
        assert!(!r.sources.is_empty(), "Should have at least one source");
    }
}

#[tokio::test]
async fn test_hybrid_search_custom_weights() {
    let (driver, embedder) = setup_test_graph().await;

    let weights = RetrievalWeights {
        semantic: 0.8,
        keyword: 0.1,
        traversal: 0.1,
    };
    let retriever = HybridRetriever::with_weights(weights);

    let results = retriever
        .search(
            "Anthropic",
            3,
            None,
            None,
            embedder.as_ref(),
            driver.as_ref(),
        )
        .await
        .unwrap();

    assert!(!results.is_empty(), "Should find Anthropic");
}

#[tokio::test]
async fn test_hybrid_search_with_seeds() {
    let (driver, embedder) = setup_test_graph().await;

    // Get Alice's ID for seed
    let embedding = embedder.embed_one("Alice").await.unwrap();
    let search_results = driver
        .search_entities_by_vector(&embedding, 1, None)
        .await
        .unwrap();

    if let Some(alice) = search_results.first() {
        let retriever = HybridRetriever::new();
        let seed_ids = vec![alice.entity.id.clone()];

        let results = retriever
            .search(
                "researcher",
                5,
                Some(&seed_ids),
                None,
                embedder.as_ref(),
                driver.as_ref(),
            )
            .await
            .unwrap();

        // With seeds, traversal should also contribute
        assert!(
            !results.is_empty(),
            "Should find results with seed-based traversal"
        );
    }
}

// ─── RRF Fusion Tests ─────────────────────────────────────────

#[test]
fn test_rrf_weights_sum_to_one() {
    let weights = RetrievalWeights::default();
    let total = weights.semantic + weights.keyword + weights.traversal;
    assert!(
        (total - 1.0).abs() < f64::EPSILON,
        "Default weights should sum to 1.0, got {}",
        total
    );
}

// ─── Search Recipes Tests ─────────────────────────────────────

#[tokio::test]
async fn test_recipe_find_entity() {
    let (driver, embedder) = setup_test_graph().await;

    let entity = SearchRecipes::find_entity("Anthropic", embedder.as_ref(), driver.as_ref()).await;

    assert!(entity.is_some(), "Should find Anthropic");
    if let Some(e) = entity {
        assert_eq!(e.name, "Anthropic");
    }
}

#[tokio::test]
async fn test_recipe_find_related() {
    let (driver, embedder) = setup_test_graph().await;

    let results =
        SearchRecipes::find_related("AI safety", 5, embedder.as_ref(), driver.as_ref()).await;

    // May or may not find results depending on embedder
    // But should not error
    let _ = results;
}

// ─── Context Builder Tests ────────────────────────────────────

#[test]
fn test_context_builder_with_entities_and_relationships() {
    let builder = ContextBuilder::new(4000);

    let alice = Entity::new("Alice", "Person").with_summary("A researcher");
    let anthropic = Entity::new("Anthropic", "Organization").with_summary("AI safety company");

    let rel = Relationship::new(
        alice.id.clone(),
        anthropic.id.clone(),
        "WORKS_AT",
        "Alice works at Anthropic",
    );

    let ctx = builder.build(&[alice, anthropic], &[rel], &[], "Where does Alice work?");

    assert!(
        ctx.context_text.contains("Alice"),
        "Context should mention Alice"
    );
    assert!(
        ctx.context_text.contains("Anthropic"),
        "Context should mention Anthropic"
    );
    assert_eq!(ctx.entity_count, 2);
    assert_eq!(ctx.relationship_count, 1);
    assert!(ctx.estimated_tokens > 0);
}

#[test]
fn test_context_builder_empty() {
    let builder = ContextBuilder::new(4000);
    let ctx = builder.build(&[], &[], &[], "test");
    assert!(ctx.context_text.contains("No relevant information"));
    assert_eq!(ctx.entity_count, 0);
}

#[test]
fn test_context_builder_token_budget() {
    let builder = ContextBuilder::new(10); // Very small budget

    let entities: Vec<Entity> = (0..50)
        .map(|i| {
            Entity::new(format!("Entity{}", i), "Test").with_summary(
                "A very long summary that takes up lots of tokens in the context window",
            )
        })
        .collect();

    let ctx = builder.build(&entities, &[], &[], "test");
    assert!(
        ctx.entity_count < 50,
        "Should truncate at budget: got {}",
        ctx.entity_count
    );
}

// ─── Query Router Tests ───────────────────────────────────────

#[test]
fn test_query_classification_local() {
    let router = QueryRouter::new();
    assert_eq!(router.classify("Where does Alice work?"), QueryType::Local);
    assert_eq!(router.classify("Who is Bob?"), QueryType::Local);
    assert_eq!(router.classify("Tell me about Anthropic"), QueryType::Local);
}

#[test]
fn test_query_classification_global() {
    let router = QueryRouter::new();
    assert_eq!(
        router.classify("Summarize all the topics"),
        QueryType::Global
    );
    assert_eq!(
        router.classify("Give me an overview of everything"),
        QueryType::Global
    );
    assert_eq!(
        router.classify("What are the main themes?"),
        QueryType::Global
    );
}

#[test]
fn test_query_classification_temporal() {
    let router = QueryRouter::new();
    assert_eq!(router.classify("When did Alice move?"), QueryType::Temporal);
    assert_eq!(
        router.classify("What was true before 2020?"),
        QueryType::Temporal
    );
    assert_eq!(router.classify("Show me the history"), QueryType::Temporal);
}

#[test]
fn test_query_classification_multihop() {
    let router = QueryRouter::new();
    assert_eq!(
        router.classify("How is Alice connected to Bob?"),
        QueryType::MultiHop
    );
    assert_eq!(
        router.classify("What is the relationship between X and Y?"),
        QueryType::MultiHop
    );
}

// ─── Budget Tests ─────────────────────────────────────────────

#[test]
fn test_budget_tracking() {
    let budget = QueryBudget::new(Some(0.10));

    assert!(budget.record_cost(0.03).is_ok());
    assert!(budget.record_cost(0.04).is_ok());
    assert!((budget.current_cost_usd() - 0.07).abs() < 0.001);

    assert!(budget.record_cost(0.05).is_err(), "Should exceed budget");
}

#[test]
fn test_budget_unlimited() {
    let budget = QueryBudget::new(None);
    assert!(budget.record_cost(100.0).is_ok());
    assert!(budget.can_afford(1000.0));
}

#[test]
fn test_budget_can_afford() {
    let budget = QueryBudget::new(Some(0.10));
    assert!(budget.can_afford(0.05));
    budget.record_cost(0.08).unwrap();
    assert!(
        !budget.can_afford(0.05),
        "Should not afford after spending most of budget"
    );
}

// ─── Full Query Engine Tests ──────────────────────────────────

#[tokio::test]
async fn test_query_engine_local() {
    let ng = neurograph_core::NeuroGraph::builder()
        .build()
        .await
        .unwrap();

    // Ingest some data
    ng.add_text("Alice works at Anthropic as a researcher")
        .await
        .unwrap();
    ng.add_text("Bob founded Anthropic in 2021").await.unwrap();

    // Query
    let result = ng.query("Who works at Anthropic?").await.unwrap();

    assert!(!result.answer.is_empty(), "Should produce an answer");

    // Without LLM, the answer should contain raw context
    assert!(
        result.answer.contains("Alice")
            || result.answer.contains("Anthropic")
            || result.answer.contains("No relevant"),
        "Answer should reference entities or indicate no results"
    );
}

#[tokio::test]
async fn test_query_engine_empty_graph() {
    let ng = neurograph_core::NeuroGraph::builder()
        .build()
        .await
        .unwrap();

    let result = ng.query("Who is Alice?").await.unwrap();

    assert!(
        !result.answer.is_empty(),
        "Should provide an answer even for empty graph"
    );
}

#[tokio::test]
async fn test_query_engine_search() {
    let ng = neurograph_core::NeuroGraph::builder()
        .build()
        .await
        .unwrap();

    ng.add_text("Alice works at Anthropic").await.unwrap();

    let entities = ng.search_entities("Alice", 5).await.unwrap();
    // May find Alice depending on embedding similarity
    let _ = entities;
}
