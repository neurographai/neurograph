// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Temporal scenario tests — comprehensive validation of bi-temporal correctness.

use chrono::{Duration, Utc};
use neurograph_core::drivers::memory::MemoryDriver;
use neurograph_core::drivers::traits::GraphDriver;
use neurograph_core::graph::{Entity, EntityId, Relationship};
use neurograph_core::temporal::manager::TemporalManager;
use neurograph_core::temporal::versioning::{
    EntityChangeType, EntityHistory, FactVersion, FactVersionChain,
};
use neurograph_core::temporal::forgetting::{ForgettingConfig, ForgettingEngine};
use neurograph_core::NeuroGraph;
use std::sync::Arc;

// ============================================================
// POINT-IN-TIME SNAPSHOT TESTS
// ============================================================

#[tokio::test]
async fn test_snapshot_empty_graph() {
    let driver = Arc::new(MemoryDriver::new());
    let mgr = TemporalManager::new(driver);
    let snapshot = mgr.snapshot_at(Utc::now(), None).await.unwrap();
    assert_eq!(snapshot.entity_count, 0);
    assert_eq!(snapshot.relationship_count, 0);
}

#[tokio::test]
async fn test_snapshot_includes_entities_created_before() {
    let driver = Arc::new(MemoryDriver::new());
    let entity = Entity::new("Alice", "Person");
    driver.store_entity(&entity).await.unwrap();

    let mgr = TemporalManager::new(driver);
    let future = Utc::now() + Duration::hours(1);
    let snapshot = mgr.snapshot_at(future, None).await.unwrap();
    assert_eq!(snapshot.entity_count, 1);
    assert_eq!(snapshot.entities[0].name, "Alice");
}

#[tokio::test]
async fn test_snapshot_excludes_entities_created_after() {
    let driver = Arc::new(MemoryDriver::new());
    let mut entity = Entity::new("Future Alice", "Person");
    entity.created_at = Utc::now() + Duration::days(30);
    driver.store_entity(&entity).await.unwrap();

    let mgr = TemporalManager::new(driver);
    let snapshot = mgr.snapshot_at(Utc::now(), None).await.unwrap();
    assert_eq!(snapshot.entity_count, 0, "Future entity should not appear");
}

#[tokio::test]
async fn test_snapshot_filters_by_valid_time() {
    let driver = Arc::new(MemoryDriver::new());
    let alice = Entity::new("Alice", "Person");
    let bob = Entity::new("Bob", "Person");
    driver.store_entity(&alice).await.unwrap();
    driver.store_entity(&bob).await.unwrap();

    let yesterday = Utc::now() - Duration::days(1);
    let tomorrow = Utc::now() + Duration::days(1);

    // Relationship valid from yesterday (should appear in current snapshot)
    let rel1 = Relationship::new(
        alice.id.clone(), bob.id.clone(),
        "KNOWS", "Alice knows Bob",
    ).with_valid_from(yesterday);
    driver.store_relationship(&rel1).await.unwrap();

    // Relationship valid from tomorrow (should NOT appear in current snapshot)
    let rel2 = Relationship::new(
        bob.id.clone(), alice.id.clone(),
        "MENTORS", "Bob mentors Alice",
    ).with_valid_from(tomorrow);
    driver.store_relationship(&rel2).await.unwrap();

    let mgr = TemporalManager::new(driver);
    let snapshot = mgr.snapshot_at(Utc::now(), None).await.unwrap();
    assert_eq!(snapshot.relationship_count, 1);
    assert_eq!(snapshot.relationships[0].fact, "Alice knows Bob");
}

#[tokio::test]
async fn test_snapshot_multiple_timestamps() {
    let driver = Arc::new(MemoryDriver::new());
    let now = Utc::now();

    // Create entities at different times
    let mut e1 = Entity::new("Early", "Event");
    e1.created_at = now - Duration::days(10);
    driver.store_entity(&e1).await.unwrap();

    let mut e2 = Entity::new("Middle", "Event");
    e2.created_at = now - Duration::days(5);
    driver.store_entity(&e2).await.unwrap();

    let mut e3 = Entity::new("Recent", "Event");
    e3.created_at = now - Duration::days(1);
    driver.store_entity(&e3).await.unwrap();

    let mgr = TemporalManager::new(driver);

    // 7 days ago: should see Early only
    let snap1 = mgr.snapshot_at(now - Duration::days(7), None).await.unwrap();
    assert_eq!(snap1.entity_count, 1);
    assert_eq!(snap1.entities[0].name, "Early");

    // 3 days ago: should see Early + Middle
    let snap2 = mgr.snapshot_at(now - Duration::days(3), None).await.unwrap();
    assert_eq!(snap2.entity_count, 2);

    // Now: should see all 3
    let snap3 = mgr.snapshot_at(now, None).await.unwrap();
    assert_eq!(snap3.entity_count, 3);
}

// ============================================================
// WHAT_CHANGED TESTS
// ============================================================

#[tokio::test]
async fn test_what_changed_empty_range() {
    let driver = Arc::new(MemoryDriver::new());
    let mgr = TemporalManager::new(driver);
    let from = Utc::now() - Duration::days(10);
    let to = Utc::now() - Duration::days(5);
    let diff = mgr.what_changed(from, to, None).await.unwrap();
    assert!(diff.added_entities.is_empty());
    assert!(diff.modified_entities.is_empty());
}

#[tokio::test]
async fn test_what_changed_detects_new_entities() {
    let driver = Arc::new(MemoryDriver::new());
    let before = Utc::now();
    let entity = Entity::new("NewEntity", "Test");
    driver.store_entity(&entity).await.unwrap();
    let after = Utc::now();

    let mgr = TemporalManager::new(driver);
    let diff = mgr.what_changed(before, after, None).await.unwrap();
    assert_eq!(diff.added_entities.len(), 1);
    assert_eq!(diff.added_entities[0].name, "NewEntity");
}

#[tokio::test]
async fn test_what_changed_excludes_outside_range() {
    let driver = Arc::new(MemoryDriver::new());
    let entity = Entity::new("OldEntity", "Test");
    driver.store_entity(&entity).await.unwrap();

    let mgr = TemporalManager::new(driver);
    // Query a future range
    let from = Utc::now() + Duration::days(1);
    let to = Utc::now() + Duration::days(2);
    let diff = mgr.what_changed(from, to, None).await.unwrap();
    assert!(diff.added_entities.is_empty());
}

// ============================================================
// TIMELINE BUILDING TESTS
// ============================================================

#[tokio::test]
async fn test_build_timeline_empty() {
    let driver = Arc::new(MemoryDriver::new());
    let mgr = TemporalManager::new(driver);
    let timeline = mgr.build_timeline(None).await.unwrap();
    assert!(timeline.is_empty());
}

#[tokio::test]
async fn test_build_timeline_groups_by_date() {
    let driver = Arc::new(MemoryDriver::new());
    driver.store_entity(&Entity::new("A", "Test")).await.unwrap();
    driver.store_entity(&Entity::new("B", "Test")).await.unwrap();
    driver.store_entity(&Entity::new("C", "Test")).await.unwrap();

    let mgr = TemporalManager::new(driver);
    let timeline = mgr.build_timeline(None).await.unwrap();
    assert!(!timeline.is_empty());
    // All created today, so should be grouped into 1 event
    assert_eq!(timeline.len(), 1);
    assert_eq!(timeline[0].entity_count, 3);
}

#[tokio::test]
async fn test_build_timeline_chronological_order() {
    let driver = Arc::new(MemoryDriver::new());
    let now = Utc::now();

    let mut e1 = Entity::new("Old", "Test");
    e1.created_at = now - Duration::days(30);
    driver.store_entity(&e1).await.unwrap();

    let e2 = Entity::new("Recent", "Test");
    driver.store_entity(&e2).await.unwrap();

    let mgr = TemporalManager::new(driver);
    let timeline = mgr.build_timeline(None).await.unwrap();
    assert!(timeline.len() >= 1);
    // Should be sorted chronologically
    for i in 1..timeline.len() {
        assert!(timeline[i].timestamp >= timeline[i - 1].timestamp);
    }
}

// ============================================================
// DATE PARSING TESTS
// ============================================================

#[test]
fn test_parse_all_supported_formats() {
    let formats = vec![
        ("2025-06-15", "2025-06-15"),
        ("2025-06-15T10:30:00Z", "2025-06-15"),
        ("2025", "2025-01-01"),
        ("2025/06/15", "2025-06-15"),
    ];

    for (input, expected_date) in formats {
        let result = TemporalManager::parse_date(input);
        assert!(
            result.is_ok(),
            "Failed to parse '{}': {:?}",
            input,
            result.err()
        );
        let dt = result.unwrap();
        assert_eq!(
            dt.date_naive().to_string(),
            expected_date,
            "Wrong date for input '{}'",
            input
        );
    }
}

#[test]
fn test_parse_invalid_dates() {
    let invalid = vec![
        "not a date",
        "",
        "abc-de-fg",
        "99999",
    ];

    for input in invalid {
        assert!(
            TemporalManager::parse_date(input).is_err(),
            "Should fail for '{}'",
            input
        );
    }
}

// ============================================================
// FACT VERSION CHAIN TESTS
// ============================================================

#[test]
fn test_version_chain_empty() {
    let chain = FactVersionChain::new(
        EntityId::new(), EntityId::new(), "WORKS_AT",
    );
    assert_eq!(chain.version_count(), 0);
    assert!(chain.current().is_none());
}

#[test]
fn test_version_chain_single_version() {
    let src = EntityId::new();
    let tgt = EntityId::new();
    let mut chain = FactVersionChain::new(src, tgt, "WORKS_AT");

    let now = Utc::now();
    chain.add_version(FactVersion {
        relationship_id: neurograph_core::graph::RelationshipId::new(),
        version: 1,
        fact: "Alice works at Google".to_string(),
        valid_from: now,
        valid_until: None,
        created_at: now,
        superseded_at: None,
        superseded_by: None,
        confidence: 0.95,
    });

    assert_eq!(chain.version_count(), 1);
    let current = chain.current().unwrap();
    assert_eq!(current.fact, "Alice works at Google");
    assert_eq!(current.confidence, 0.95);
}

#[test]
fn test_version_chain_supersession() {
    let src = EntityId::new();
    let tgt = EntityId::new();
    let mut chain = FactVersionChain::new(src, tgt, "WORKS_AT");

    let past = Utc::now() - Duration::days(365);
    let now = Utc::now();

    chain.add_version(FactVersion {
        relationship_id: neurograph_core::graph::RelationshipId::new(),
        version: 1,
        fact: "Alice works at Google".to_string(),
        valid_from: past,
        valid_until: Some(now),
        created_at: past,
        superseded_at: Some(now),
        superseded_by: None,
        confidence: 0.9,
    });

    chain.add_version(FactVersion {
        relationship_id: neurograph_core::graph::RelationshipId::new(),
        version: 2,
        fact: "Alice works at Anthropic".to_string(),
        valid_from: now,
        valid_until: None,
        created_at: now,
        superseded_at: None,
        superseded_by: None,
        confidence: 0.95,
    });

    assert_eq!(chain.version_count(), 2);
    assert_eq!(chain.current().unwrap().fact, "Alice works at Anthropic");

    // Point-in-time query
    let six_months_ago = Utc::now() - Duration::days(180);
    let historical = chain.version_at(&six_months_ago).unwrap();
    assert_eq!(historical.fact, "Alice works at Google");
}

// ============================================================
// ENTITY HISTORY TESTS
// ============================================================

#[test]
fn test_entity_history_empty() {
    let history = EntityHistory::new(EntityId::new());
    assert!(history.entries.is_empty());
    assert!(history.latest().is_none());
}

#[test]
fn test_entity_history_record_and_retrieve() {
    let mut history = EntityHistory::new(EntityId::new());
    history.record(EntityChangeType::Created, "Entity created");
    history.record(EntityChangeType::SummaryUpdated, "Summary updated");
    history.record(EntityChangeType::Merged, "Merged with duplicate");

    assert_eq!(history.entries.len(), 3);
    assert!(matches!(
        history.latest().unwrap().change_type,
        EntityChangeType::Merged
    ));
}

#[test]
fn test_entity_history_changes_between() {
    let mut history = EntityHistory::new(EntityId::new());
    let t1 = Utc::now() - Duration::hours(3);
    let t2 = Utc::now() - Duration::hours(1);

    history.entries.push(neurograph_core::temporal::versioning::EntityHistoryEntry {
        timestamp: Utc::now() - Duration::hours(4),
        change_type: EntityChangeType::Created,
        description: "Created".to_string(),
        old_summary: None,
        new_summary: None,
    });
    history.entries.push(neurograph_core::temporal::versioning::EntityHistoryEntry {
        timestamp: Utc::now() - Duration::hours(2),
        change_type: EntityChangeType::SummaryUpdated,
        description: "Updated".to_string(),
        old_summary: None,
        new_summary: Some("New summary".to_string()),
    });

    let changes = history.changes_between(&t1, &t2);
    assert_eq!(changes.len(), 1);
    assert!(matches!(
        changes[0].change_type,
        EntityChangeType::SummaryUpdated
    ));
}

// ============================================================
// FORGETTING ENGINE TESTS
// ============================================================

#[tokio::test]
async fn test_forgetting_disabled_by_default() {
    let driver = Arc::new(MemoryDriver::new());
    let config = ForgettingConfig::default();
    let engine = ForgettingEngine::new(config, driver);
    let result = engine.decay_pass(false, None).await.unwrap();
    assert_eq!(result.total_evaluated, 0);
}

#[tokio::test]
async fn test_forgetting_identifies_low_importance() {
    let driver = Arc::new(MemoryDriver::new());

    let mut unimportant = Entity::new("Unimportant", "Test");
    unimportant.importance_score = 0.01;
    unimportant.access_count = 0;
    driver.store_entity(&unimportant).await.unwrap();

    let mut important = Entity::new("Important", "Test");
    important.importance_score = 0.9;
    important.access_count = 100;
    driver.store_entity(&important).await.unwrap();

    let config = ForgettingConfig {
        enabled: true,
        importance_threshold: 0.1,
        min_access_count: 5,
        ..Default::default()
    };
    let engine = ForgettingEngine::new(config, driver);

    let candidates = engine.get_prune_candidates(None, 100).await.unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].name, "Unimportant");
}

#[tokio::test]
async fn test_forgetting_auto_prune() {
    let driver = Arc::new(MemoryDriver::new());

    let mut entity = Entity::new("Prunable", "Test");
    entity.importance_score = 0.01;
    entity.access_count = 0;
    entity.updated_at = Utc::now() - Duration::days(30);
    driver.store_entity(&entity).await.unwrap();

    let config = ForgettingConfig {
        enabled: true,
        importance_threshold: 0.5,
        min_access_count: 5,
        daily_decay_rate: 0.1,
        ..Default::default()
    };
    let engine = ForgettingEngine::new(config, driver.clone());
    let result = engine.decay_pass(true, None).await.unwrap();

    assert!(result.pruned > 0, "Should auto-prune low importance entity");
    let remaining = driver.list_entities(None, 100).await.unwrap();
    assert_eq!(remaining.len(), 0);
}

#[test]
fn test_ttl_not_expired() {
    let driver = Arc::new(MemoryDriver::new());
    let config = ForgettingConfig {
        enabled: true,
        default_ttl: Some(Duration::days(30)),
        ..Default::default()
    };
    let engine = ForgettingEngine::new(config, driver);

    let entity = Entity::new("Fresh", "Test");
    assert!(!engine.is_expired(&entity));
}

#[test]
fn test_ttl_expired() {
    let driver = Arc::new(MemoryDriver::new());
    let config = ForgettingConfig {
        enabled: true,
        default_ttl: Some(Duration::days(7)),
        ..Default::default()
    };
    let engine = ForgettingEngine::new(config, driver);

    let mut old = Entity::new("Old", "Test");
    old.created_at = Utc::now() - Duration::days(10);
    assert!(engine.is_expired(&old));
}

// ============================================================
// NEUROGRAPH API TEMPORAL TESTS
// ============================================================

#[tokio::test]
async fn test_ng_at_returns_view() {
    let ng = NeuroGraph::builder().memory().build().await.unwrap();
    ng.store_entity(&Entity::new("Alice", "Person")).await.unwrap();

    let view = ng.at("2030-01-01").await.unwrap();
    assert_eq!(view.entity_count(), 1);
}

#[tokio::test]
async fn test_ng_at_past_excludes_future_entities() {
    let ng = NeuroGraph::builder().memory().build().await.unwrap();
    let mut future_entity = Entity::new("Future", "Test");
    future_entity.created_at = Utc::now() + Duration::days(365);
    ng.store_entity(&future_entity).await.unwrap();

    let view = ng.at("2026-01-01").await.unwrap();
    // Entity created in the future shouldn't appear
    assert_eq!(view.entity_count(), 0);
}

#[tokio::test]
async fn test_ng_add_text_at() {
    let ng = NeuroGraph::builder().memory().build().await.unwrap();
    let episode = ng.add_text_at("Alice works at Google", "2023-01-15").await.unwrap();
    assert_eq!(
        episode.created_at.date_naive().to_string(),
        "2023-01-15"
    );
}

#[tokio::test]
async fn test_ng_what_changed() {
    let ng = NeuroGraph::builder().memory().build().await.unwrap();
    let before = Utc::now();
    ng.store_entity(&Entity::new("NewEntity", "Test")).await.unwrap();
    let after = Utc::now();

    let diff = ng
        .what_changed(
            &before.to_rfc3339(),
            &after.to_rfc3339(),
        )
        .await
        .unwrap();
    assert_eq!(diff.added_entities.len(), 1);
}

#[tokio::test]
async fn test_ng_build_timeline() {
    let ng = NeuroGraph::builder().memory().build().await.unwrap();
    ng.store_entity(&Entity::new("A", "T")).await.unwrap();
    ng.store_entity(&Entity::new("B", "T")).await.unwrap();

    let timeline = ng.build_timeline().await.unwrap();
    assert!(!timeline.is_empty());
}
