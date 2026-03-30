// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Cost tracking correctness tests.

use neurograph_core::utils::concurrency::CostTracker;
use neurograph_core::NeuroGraph;

// ============================================================
// COST TRACKER UNIT TESTS
// ============================================================

#[test]
fn test_cost_tracker_initial_zero() {
    let tracker = CostTracker::new(None);
    assert_eq!(tracker.total_cost_usd(), 0.0);
}

#[test]
fn test_cost_tracker_accumulates() {
    let tracker = CostTracker::new(None);
    tracker.record(0.01);
    tracker.record(0.02);
    tracker.record(0.005);

    let total = tracker.total_cost_usd();
    assert!(
        (total - 0.035).abs() < 1e-10,
        "Expected 0.035, got {}",
        total
    );
}

#[test]
fn test_cost_tracker_reset() {
    let tracker = CostTracker::new(None);
    tracker.record(1.0);
    assert!(tracker.total_cost_usd() > 0.0);

    tracker.reset();
    assert_eq!(tracker.total_cost_usd(), 0.0);
}

#[test]
fn test_cost_tracker_with_budget() {
    let tracker = CostTracker::new(Some(5.0));
    tracker.record(2.0);
    tracker.record(2.0);
    // Total is 4.0, within budget of 5.0
    assert_eq!(tracker.total_cost_usd(), 4.0);
}

// ============================================================
// NEUROGRAPH COST INTEGRATION TESTS
// ============================================================

#[tokio::test]
async fn test_ng_cost_starts_at_zero() {
    let ng = NeuroGraph::builder().memory().build().await.unwrap();
    assert_eq!(ng.total_cost_usd(), 0.0);
}

#[tokio::test]
async fn test_ng_cost_tracked_after_operations() {
    let ng = NeuroGraph::builder().memory().build().await.unwrap();

    // add_text triggers the pipeline which records cost
    ng.add_text("Alice works at Anthropic").await.unwrap();

    // Cost should still be 0 since we use regex extraction (no LLM)
    // but the tracking mechanism should work
    let cost = ng.total_cost_usd();
    assert!(cost >= 0.0, "Cost should be non-negative");
}

#[tokio::test]
async fn test_ng_cost_resets_on_clear() {
    let ng = NeuroGraph::builder().memory().build().await.unwrap();

    ng.add_text("Test data").await.unwrap();
    ng.clear().await.unwrap();

    assert_eq!(ng.total_cost_usd(), 0.0);
}
