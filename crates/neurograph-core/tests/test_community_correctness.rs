// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Community detection correctness tests.

use neurograph_core::community::louvain::{LouvainConfig, LouvainDetector};
use neurograph_core::drivers::memory::MemoryDriver;
use neurograph_core::drivers::traits::GraphDriver;
use neurograph_core::graph::{Entity, Relationship};
use neurograph_core::NeuroGraph;
use std::collections::HashSet;

// ============================================================
// LOUVAIN CORE ALGORITHM TESTS
// ============================================================

#[tokio::test]
async fn test_louvain_empty_graph() {
    let driver = MemoryDriver::new();
    let detector = LouvainDetector::new();
    let result = detector.detect(&driver, None).await.unwrap();
    assert!(result.communities.is_empty());
    assert_eq!(result.modularity, 0.0);
    assert_eq!(result.levels, 0);
}

#[tokio::test]
async fn test_louvain_single_entity_no_edges() {
    let driver = MemoryDriver::new();
    driver
        .store_entity(&Entity::new("Solo", "Node"))
        .await
        .unwrap();

    let detector = LouvainDetector::new();
    let result = detector.detect(&driver, None).await.unwrap();
    assert_eq!(result.communities.len(), 1);
}

#[tokio::test]
async fn test_louvain_two_connected_entities() {
    let driver = MemoryDriver::new();
    let a = Entity::new("A", "Node");
    let b = Entity::new("B", "Node");
    driver.store_entity(&a).await.unwrap();
    driver.store_entity(&b).await.unwrap();

    let rel = Relationship::new(a.id.clone(), b.id.clone(), "KNOWS", "A knows B");
    driver.store_relationship(&rel).await.unwrap();

    let detector = LouvainDetector::new();
    let result = detector.detect(&driver, None).await.unwrap();
    // Two connected nodes should be in the same community
    assert!(!result.communities.is_empty());
}

#[tokio::test]
async fn test_louvain_two_cliques_detected() {
    let driver = MemoryDriver::new();

    // Clique 1: A, B, C (fully connected)
    let a = Entity::new("A", "Node");
    let b = Entity::new("B", "Node");
    let c = Entity::new("C", "Node");
    driver.store_entity(&a).await.unwrap();
    driver.store_entity(&b).await.unwrap();
    driver.store_entity(&c).await.unwrap();

    driver
        .store_relationship(&Relationship::new(a.id.clone(), b.id.clone(), "L", "AB"))
        .await
        .unwrap();
    driver
        .store_relationship(&Relationship::new(b.id.clone(), c.id.clone(), "L", "BC"))
        .await
        .unwrap();
    driver
        .store_relationship(&Relationship::new(a.id.clone(), c.id.clone(), "L", "AC"))
        .await
        .unwrap();

    // Clique 2: D, E, F (fully connected)
    let d = Entity::new("D", "Node");
    let e = Entity::new("E", "Node");
    let f = Entity::new("F", "Node");
    driver.store_entity(&d).await.unwrap();
    driver.store_entity(&e).await.unwrap();
    driver.store_entity(&f).await.unwrap();

    driver
        .store_relationship(&Relationship::new(d.id.clone(), e.id.clone(), "L", "DE"))
        .await
        .unwrap();
    driver
        .store_relationship(&Relationship::new(e.id.clone(), f.id.clone(), "L", "EF"))
        .await
        .unwrap();
    driver
        .store_relationship(&Relationship::new(d.id.clone(), f.id.clone(), "L", "DF"))
        .await
        .unwrap();

    // Weak bridge between cliques
    driver
        .store_relationship(
            &Relationship::new(c.id.clone(), d.id.clone(), "BRIDGE", "bridge").with_weight(0.1),
        )
        .await
        .unwrap();

    let detector = LouvainDetector::new();
    let result = detector.detect(&driver, None).await.unwrap();

    // Should detect at least 2 communities
    assert!(
        result.communities.len() >= 2,
        "Expected ≥2 communities, got {}",
        result.communities.len()
    );

    // Modularity should be positive for a graph with real structure
    assert!(
        result.modularity > 0.0,
        "Modularity should be positive: {}",
        result.modularity
    );
}

#[tokio::test]
async fn test_louvain_resolution_affects_community_count() {
    let driver = MemoryDriver::new();

    // Create 4 small cliques
    for clique in 0..4 {
        let mut entities = Vec::new();
        for i in 0..4 {
            let e = Entity::new(&format!("C{}_{}", clique, i), "Node");
            driver.store_entity(&e).await.unwrap();
            entities.push(e);
        }
        // Fully connect within clique
        for i in 0..4 {
            for j in (i + 1)..4 {
                let rel = Relationship::new(
                    entities[i].id.clone(),
                    entities[j].id.clone(),
                    "INTRA",
                    &format!("intra_{}_{}", i, j),
                );
                driver.store_relationship(&rel).await.unwrap();
            }
        }
    }

    // Low resolution → fewer, larger communities
    let low_res = LouvainConfig {
        resolution: 0.5,
        ..Default::default()
    };
    let detector_low = LouvainDetector::with_config(low_res);
    let result_low = detector_low.detect(&driver, None).await.unwrap();

    // High resolution → more, smaller communities
    let high_res = LouvainConfig {
        resolution: 2.0,
        ..Default::default()
    };
    let detector_high = LouvainDetector::with_config(high_res);
    let result_high = detector_high.detect(&driver, None).await.unwrap();

    // Higher resolution should yield >= as many communities
    assert!(
        result_high.communities.len() >= result_low.communities.len(),
        "Higher resolution ({}) should give >= communities than lower ({})",
        result_high.communities.len(),
        result_low.communities.len()
    );
}

#[tokio::test]
async fn test_louvain_modularity_bounded() {
    let driver = MemoryDriver::new();
    let a = Entity::new("X", "Node");
    let b = Entity::new("Y", "Node");
    driver.store_entity(&a).await.unwrap();
    driver.store_entity(&b).await.unwrap();
    driver
        .store_relationship(&Relationship::new(a.id.clone(), b.id.clone(), "L", "XY"))
        .await
        .unwrap();

    let detector = LouvainDetector::new();
    let result = detector.detect(&driver, None).await.unwrap();
    assert!(
        result.modularity >= -0.5 && result.modularity <= 1.0,
        "Modularity {} out of expected range [-0.5, 1.0]",
        result.modularity
    );
}

#[tokio::test]
async fn test_louvain_assignments_cover_all_entities() {
    let driver = MemoryDriver::new();
    let mut entity_ids = Vec::new();
    for i in 0..10 {
        let e = Entity::new(&format!("N{}", i), "Node");
        entity_ids.push(e.id.as_str());
        driver.store_entity(&e).await.unwrap();
    }
    // Connect pairs
    let entities = driver.list_entities(None, 100).await.unwrap();
    for i in 0..5 {
        let rel = Relationship::new(
            entities[i * 2].id.clone(),
            entities[i * 2 + 1].id.clone(),
            "PAIR",
            &format!("pair_{}", i),
        );
        driver.store_relationship(&rel).await.unwrap();
    }

    let detector = LouvainDetector::new();
    let result = detector.detect(&driver, None).await.unwrap();

    // Every entity should be assigned to exactly one community
    let assigned: HashSet<String> = result.assignments.keys().cloned().collect();
    assert_eq!(
        assigned.len(),
        10,
        "All 10 entities should have community assignments"
    );
}

// ============================================================
// NEUROGRAPH COMMUNITY API TESTS
// ============================================================

#[tokio::test]
async fn test_ng_detect_communities() {
    let ng = NeuroGraph::builder().memory().build().await.unwrap();

    let alice = Entity::new("Alice", "Person");
    let bob = Entity::new("Bob", "Person");
    let company = Entity::new("Acme", "Organization");

    ng.store_entity(&alice).await.unwrap();
    ng.store_entity(&bob).await.unwrap();
    ng.store_entity(&company).await.unwrap();

    ng.store_relationship(&Relationship::new(
        alice.id.clone(),
        company.id.clone(),
        "WORKS_AT",
        "Alice works at Acme",
    ))
    .await
    .unwrap();
    ng.store_relationship(&Relationship::new(
        bob.id.clone(),
        company.id.clone(),
        "WORKS_AT",
        "Bob works at Acme",
    ))
    .await
    .unwrap();

    let result = ng.detect_communities().await.unwrap();
    assert!(!result.communities.is_empty());
    assert!(result.iterations > 0);
}

#[tokio::test]
async fn test_ng_detect_communities_with_config() {
    let ng = NeuroGraph::builder().memory().build().await.unwrap();

    for i in 0..6 {
        ng.store_entity(&Entity::new(&format!("Node_{}", i), "Node"))
            .await
            .unwrap();
    }

    let config = LouvainConfig {
        resolution: 1.5,
        max_iterations: 50,
        ..Default::default()
    };

    let result = ng.detect_communities_with(config).await.unwrap();
    assert!(result.iterations <= 50);
}

#[tokio::test]
async fn test_ng_detect_communities_empty_graph() {
    let ng = NeuroGraph::builder().memory().build().await.unwrap();
    let result = ng.detect_communities().await.unwrap();
    assert!(result.communities.is_empty());
    assert_eq!(result.modularity, 0.0);
}
