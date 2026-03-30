// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for community detection.

use std::sync::Arc;

use neurograph_core::{
    community::{
        incremental::IncrementalCommunityUpdater,
        leiden::{LeidenConfig, LeidenDetector},
        louvain::LouvainDetector,
    },
    drivers::memory::MemoryDriver,
    graph::{Community, Entity, Relationship},
    GraphDriver,
};

/// Helper: creates two clusters (Alice/Bob/Carol) and (Dave/Eve/Frank) with a bridge.
async fn setup_two_clusters(driver: &MemoryDriver) {
    let alice = Entity::new("Alice", "Person");
    let bob = Entity::new("Bob", "Person");
    let carol = Entity::new("Carol", "Person");
    let dave = Entity::new("Dave", "Person");
    let eve = Entity::new("Eve", "Person");
    let frank = Entity::new("Frank", "Person");

    for entity in [&alice, &bob, &carol, &dave, &eve, &frank] {
        let _: () = driver.store_entity(entity).await.unwrap();
    }

    let rels = vec![
        Relationship::new(alice.id.clone(), bob.id.clone(), "KNOWS", "Alice knows Bob"),
        Relationship::new(bob.id.clone(), carol.id.clone(), "KNOWS", "Bob knows Carol"),
        Relationship::new(
            alice.id.clone(),
            carol.id.clone(),
            "KNOWS",
            "Alice knows Carol",
        ),
        Relationship::new(dave.id.clone(), eve.id.clone(), "KNOWS", "Dave knows Eve"),
        Relationship::new(eve.id.clone(), frank.id.clone(), "KNOWS", "Eve knows Frank"),
        Relationship::new(
            dave.id.clone(),
            frank.id.clone(),
            "KNOWS",
            "Dave knows Frank",
        ),
        Relationship::new(
            carol.id.clone(),
            dave.id.clone(),
            "KNOWS",
            "Carol knows Dave",
        ),
    ];

    for rel in &rels {
        let _: () = driver.store_relationship(rel).await.unwrap();
    }
}

#[tokio::test]
async fn test_louvain_detects_communities() {
    let driver = MemoryDriver::new();
    setup_two_clusters(&driver).await;

    let detector = LouvainDetector::new();
    let result = detector.detect(&driver, None).await.unwrap();

    assert!(
        !result.communities.is_empty(),
        "Should detect at least one community"
    );
    assert!(result.modularity >= 0.0);
}

#[tokio::test]
async fn test_leiden_detects_communities() {
    let driver = MemoryDriver::new();
    setup_two_clusters(&driver).await;

    let detector = LeidenDetector::new();
    let result = detector.detect(&driver, None).await.unwrap();

    assert!(
        result.communities.len() >= 2,
        "Should find at least 2 communities, found {}",
        result.communities.len()
    );
    assert!(result.modularity >= 0.0);
    assert!(result.iterations > 0);
}

#[tokio::test]
async fn test_leiden_empty_graph() {
    let driver = MemoryDriver::new();
    let detector = LeidenDetector::new();
    let result = detector.detect(&driver, None).await.unwrap();
    assert!(result.communities.is_empty());
    assert_eq!(result.modularity, 0.0);
}

#[tokio::test]
async fn test_leiden_custom_resolution() {
    let driver = MemoryDriver::new();
    setup_two_clusters(&driver).await;

    let config = LeidenConfig {
        resolution: 2.0,
        max_iterations: 10,
        ..Default::default()
    };

    let detector = LeidenDetector::with_config(config);
    let result = detector.detect(&driver, None).await.unwrap();
    assert!(!result.communities.is_empty());
}

#[tokio::test]
async fn test_incremental_community_update() {
    let driver = Arc::new(MemoryDriver::new());

    // Store an entity not in any community
    let alice = Entity::new("Alice", "Person");
    driver.store_entity(&alice).await.unwrap();

    let updater = IncrementalCommunityUpdater::new(driver.clone());
    let result = updater
        .update_after_ingestion(&[alice.id.clone()], None)
        .await
        .unwrap();

    // Should create a new singleton community
    assert_eq!(result.new_communities, 1);
    assert_eq!(result.entities_processed, 1);
}

#[tokio::test]
async fn test_incremental_with_existing_community() {
    let driver = Arc::new(MemoryDriver::new());

    let alice = Entity::new("Alice", "Person");
    driver.store_entity(&alice).await.unwrap();

    // Create a community containing Alice
    let mut community = Community::new("test-comm", 0);
    community.add_member(alice.id.clone());
    driver.store_community(&community).await.unwrap();

    // Now add Bob who is connected to Alice
    let bob = Entity::new("Bob", "Person");
    driver.store_entity(&bob).await.unwrap();

    let rel = Relationship::new(bob.id.clone(), alice.id.clone(), "KNOWS", "Bob knows Alice");
    driver.store_relationship(&rel).await.unwrap();

    let updater = IncrementalCommunityUpdater::new(driver.clone());
    let result = updater
        .update_after_ingestion(&[bob.id.clone()], None)
        .await
        .unwrap();

    assert_eq!(result.entities_processed, 1);
}

#[tokio::test]
async fn test_communities_stored_in_driver() {
    let driver = MemoryDriver::new();
    setup_two_clusters(&driver).await;

    let detector = LeidenDetector::new();
    let _result = detector.detect(&driver, None).await.unwrap();

    let communities = driver.list_communities(None).await.unwrap();
    assert!(
        !communities.is_empty(),
        "Communities should be persisted in driver"
    );
}
