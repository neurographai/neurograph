// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Driver-agnostic tests that run against both Memory and Embedded drivers.

use neurograph_core::drivers::embedded::EmbeddedDriver;
use neurograph_core::drivers::memory::MemoryDriver;
use neurograph_core::drivers::traits::GraphDriver;
use neurograph_core::graph::{Community, Entity, EntityId, Episode, Relationship};

/// Run a test function against both drivers.
async fn run_against_both<F, Fut>(test_fn: F)
where
    F: Fn(Box<dyn GraphDriver>) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    // Memory driver
    let memory = Box::new(MemoryDriver::new()) as Box<dyn GraphDriver>;
    test_fn(memory).await;

    // Embedded driver (temporary)
    let embedded = Box::new(EmbeddedDriver::temporary().unwrap()) as Box<dyn GraphDriver>;
    test_fn(embedded).await;
}

#[tokio::test]
async fn test_driver_entity_roundtrip() {
    run_against_both(|driver| async move {
        let entity = Entity::new("TestEntity", "TestType")
            .with_summary("A test entity")
            .with_embedding(vec![1.0, 2.0, 3.0]);

        driver.store_entity(&entity).await.unwrap();

        let retrieved = driver.get_entity(&entity.id).await.unwrap();
        assert_eq!(retrieved.name, "TestEntity");
        assert_eq!(retrieved.entity_type.as_str(), "TestType");
        assert_eq!(retrieved.summary, "A test entity");
        assert!(retrieved.name_embedding.is_some());
    })
    .await;
}

#[tokio::test]
async fn test_driver_entity_not_found() {
    run_against_both(|driver| async move {
        let result = driver.get_entity(&EntityId::new()).await;
        assert!(result.is_err());
    })
    .await;
}

#[tokio::test]
async fn test_driver_entity_delete() {
    run_against_both(|driver| async move {
        let entity = Entity::new("ToDelete", "Type");
        driver.store_entity(&entity).await.unwrap();
        assert!(driver.get_entity(&entity.id).await.is_ok());

        driver.delete_entity(&entity.id).await.unwrap();
        assert!(driver.get_entity(&entity.id).await.is_err());
    })
    .await;
}

#[tokio::test]
async fn test_driver_relationship_roundtrip() {
    run_against_both(|driver| async move {
        let src = Entity::new("Src", "Type");
        let tgt = Entity::new("Tgt", "Type");
        driver.store_entity(&src).await.unwrap();
        driver.store_entity(&tgt).await.unwrap();

        let rel = Relationship::new(
            src.id.clone(),
            tgt.id.clone(),
            "REL_TYPE",
            "Source relates to Target",
        );
        driver.store_relationship(&rel).await.unwrap();

        let rels = driver.get_entity_relationships(&src.id).await.unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].fact, "Source relates to Target");
    })
    .await;
}

#[tokio::test]
async fn test_driver_episode_roundtrip() {
    run_against_both(|driver| async move {
        let ep = Episode::text("test-episode", "Some content");
        driver.store_episode(&ep).await.unwrap();

        let retrieved = driver.get_episode(&ep.id).await.unwrap();
        assert_eq!(retrieved.name, "test-episode");
        assert_eq!(retrieved.content, "Some content");
    })
    .await;
}

#[tokio::test]
async fn test_driver_community_roundtrip() {
    run_against_both(|driver| async move {
        let mut community = Community::new("test-community", 0);
        community.add_member(EntityId::new());
        community.add_member(EntityId::new());

        driver.store_community(&community).await.unwrap();

        let retrieved = driver.get_community(&community.id).await.unwrap();
        assert_eq!(retrieved.member_count(), 2);
        assert_eq!(retrieved.level, 0);
    })
    .await;
}

#[tokio::test]
async fn test_driver_vector_search() {
    run_against_both(|driver| async move {
        let e1 = Entity::new("ML", "Concept").with_embedding(vec![1.0, 0.0, 0.0]);
        let e2 = Entity::new("DL", "Concept").with_embedding(vec![0.9, 0.1, 0.0]);
        let e3 = Entity::new("Cook", "Concept").with_embedding(vec![0.0, 0.0, 1.0]);

        driver.store_entity(&e1).await.unwrap();
        driver.store_entity(&e2).await.unwrap();
        driver.store_entity(&e3).await.unwrap();

        let results = driver
            .search_entities_by_vector(&[1.0, 0.0, 0.0], 2, None)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].entity.name, "ML"); // Exact match first
        assert!(results[0].score > results[1].score);
    })
    .await;
}

#[tokio::test]
async fn test_driver_stats() {
    run_against_both(|driver| async move {
        driver.store_entity(&Entity::new("A", "T")).await.unwrap();
        driver.store_entity(&Entity::new("B", "T")).await.unwrap();

        let stats = driver.stats().await.unwrap();
        assert_eq!(stats["entities"], 2);
    })
    .await;
}

#[tokio::test]
async fn test_driver_clear() {
    run_against_both(|driver| async move {
        driver.store_entity(&Entity::new("A", "T")).await.unwrap();
        driver.clear().await.unwrap();

        let stats = driver.stats().await.unwrap();
        assert_eq!(stats["entities"], 0);
    })
    .await;
}

#[tokio::test]
async fn test_driver_list_entities_with_group() {
    run_against_both(|driver| async move {
        driver
            .store_entity(&Entity::new("A", "T").with_group_id("g1"))
            .await
            .unwrap();
        driver
            .store_entity(&Entity::new("B", "T").with_group_id("g1"))
            .await
            .unwrap();
        driver
            .store_entity(&Entity::new("C", "T").with_group_id("g2"))
            .await
            .unwrap();

        let g1 = driver.list_entities(Some("g1"), 100).await.unwrap();
        assert_eq!(g1.len(), 2);

        let g2 = driver.list_entities(Some("g2"), 100).await.unwrap();
        assert_eq!(g2.len(), 1);

        let all = driver.list_entities(None, 100).await.unwrap();
        assert_eq!(all.len(), 3);
    })
    .await;
}
