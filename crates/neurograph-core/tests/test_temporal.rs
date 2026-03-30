// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the temporal engine.

use std::sync::Arc;

use chrono::{Duration, Utc};
use neurograph_core::{
    drivers::memory::MemoryDriver,
    graph::Entity,
    temporal::{
        forgetting::{ForgettingConfig, ForgettingEngine},
        manager::TemporalManager,
        versioning::{EntityChangeType, EntityHistory, FactVersion, FactVersionChain},
    },
    GraphDriver,
};

#[tokio::test]
async fn test_temporal_snapshot_includes_entities() {
    let driver = Arc::new(MemoryDriver::new());
    let manager = TemporalManager::new(driver.clone());

    let entity = Entity::new("Alice", "Person");
    driver.store_entity(&entity).await.unwrap();

    let snapshot = manager.snapshot_at(Utc::now(), None).await.unwrap();
    assert_eq!(snapshot.entity_count, 1);
    assert_eq!(snapshot.entities[0].name, "Alice");
}

#[tokio::test]
async fn test_temporal_what_changed() {
    let driver = Arc::new(MemoryDriver::new());
    let manager = TemporalManager::new(driver.clone());

    let before = Utc::now();

    let entity = Entity::new("Bob", "Person");
    driver.store_entity(&entity).await.unwrap();

    let after = Utc::now();

    let diff = manager.what_changed(before, after, None).await.unwrap();
    assert_eq!(diff.added_entities.len(), 1);
    assert_eq!(diff.added_entities[0].name, "Bob");
}

#[tokio::test]
async fn test_temporal_parse_date_formats() {
    // ISO date
    let dt = TemporalManager::parse_date("2025-03-15").unwrap();
    assert_eq!(dt.date_naive().to_string(), "2025-03-15");

    // Year only
    let dt = TemporalManager::parse_date("2025").unwrap();
    assert_eq!(dt.date_naive().to_string(), "2025-01-01");

    // Invalid
    assert!(TemporalManager::parse_date("not-a-date").is_err());
}

#[test]
fn test_fact_version_chain_time_travel() {
    use neurograph_core::graph::entity::EntityId;
    use neurograph_core::graph::relationship::RelationshipId;

    let src = EntityId::new();
    let tgt = EntityId::new();
    let mut chain = FactVersionChain::new(src, tgt, "LIVES_IN");

    let now = Utc::now();
    let past = now - Duration::days(365);

    chain.add_version(FactVersion {
        relationship_id: RelationshipId::new(),
        version: 1,
        fact: "Alice lives in NYC".to_string(),
        valid_from: past,
        valid_until: Some(now),
        created_at: past,
        superseded_at: Some(now),
        superseded_by: None,
        confidence: 0.9,
    });

    chain.add_version(FactVersion {
        relationship_id: RelationshipId::new(),
        version: 2,
        fact: "Alice lives in SF".to_string(),
        valid_from: now,
        valid_until: None,
        created_at: now,
        superseded_at: None,
        superseded_by: None,
        confidence: 0.95,
    });

    assert_eq!(chain.version_count(), 2);
    assert_eq!(chain.current().unwrap().fact, "Alice lives in SF");

    let six_months_ago = now - Duration::days(180);
    assert_eq!(
        chain.version_at(&six_months_ago).unwrap().fact,
        "Alice lives in NYC"
    );
}

#[test]
fn test_entity_history_tracking() {
    use neurograph_core::graph::entity::EntityId;

    let id = EntityId::new();
    let mut history = EntityHistory::new(id);

    history.record(EntityChangeType::Created, "Entity created");
    history.record_summary_change("A person", "A researcher at Anthropic");

    assert_eq!(history.entries.len(), 2);
    let latest = history.latest().unwrap();
    assert!(matches!(latest.change_type, EntityChangeType::SummaryUpdated));
    assert_eq!(latest.new_summary.as_deref(), Some("A researcher at Anthropic"));
}

#[tokio::test]
async fn test_forgetting_decay_pass() {
    let driver = Arc::new(MemoryDriver::new());

    let mut low = Entity::new("LowImportance", "Test");
    low.importance_score = 0.05;
    low.access_count = 1;
    driver.store_entity(&low).await.unwrap();

    let mut high = Entity::new("HighImportance", "Test");
    high.importance_score = 0.9;
    high.access_count = 100;
    driver.store_entity(&high).await.unwrap();

    let config = ForgettingConfig {
        enabled: true,
        importance_threshold: 0.1,
        min_access_count: 5,
        ..Default::default()
    };
    let engine = ForgettingEngine::new(config, driver);

    let candidates = engine.get_prune_candidates(None, 10).await.unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].name, "LowImportance");
}

#[tokio::test]
async fn test_timeline_generation() {
    let driver = Arc::new(MemoryDriver::new());
    let manager = TemporalManager::new(driver.clone());

    driver.store_entity(&Entity::new("Alice", "Person")).await.unwrap();
    driver.store_entity(&Entity::new("Bob", "Person")).await.unwrap();

    let timeline = manager.build_timeline(None).await.unwrap();
    assert!(!timeline.is_empty());
}
