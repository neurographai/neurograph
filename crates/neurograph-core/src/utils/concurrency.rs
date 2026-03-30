// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Concurrency utilities: semaphore-based rate limiting for LLM calls.
//!
//! Influenced by Graphiti's `semaphore_gather()` pattern (helpers.py).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;

/// A rate-limited semaphore for controlling concurrent LLM API calls.
///
/// Prevents overwhelming LLM APIs with too many concurrent requests.
/// Graphiti uses a similar `SEMAPHORE_LIMIT` pattern.
#[derive(Debug, Clone)]
pub struct ConcurrencyLimiter {
    semaphore: Arc<Semaphore>,
    active_count: Arc<AtomicU64>,
    max_concurrent: usize,
}

impl ConcurrencyLimiter {
    /// Create a new limiter with the given max concurrency.
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            active_count: Arc::new(AtomicU64::new(0)),
            max_concurrent,
        }
    }

    /// Execute an async operation with concurrency limiting.
    pub async fn execute<F, T>(&self, f: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        let _permit = self.semaphore.acquire().await.expect("Semaphore closed");
        self.active_count.fetch_add(1, Ordering::Relaxed);
        let result = f.await;
        self.active_count.fetch_sub(1, Ordering::Relaxed);
        result
    }

    /// Get the current number of active operations.
    pub fn active_count(&self) -> u64 {
        self.active_count.load(Ordering::Relaxed)
    }

    /// Get the max concurrency.
    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }
}

impl Default for ConcurrencyLimiter {
    fn default() -> Self {
        Self::new(10)
    }
}

/// Cumulative cost tracker for budget enforcement.
///
/// Thread-safe accumulator for LLM costs across all operations.
#[derive(Debug, Clone)]
pub struct CostTracker {
    total_cost_microdollars: Arc<AtomicU64>,
    budget_microdollars: Option<u64>,
}

impl CostTracker {
    /// Create a new cost tracker with optional budget limit.
    pub fn new(budget_usd: Option<f64>) -> Self {
        Self {
            total_cost_microdollars: Arc::new(AtomicU64::new(0)),
            budget_microdollars: budget_usd.map(|b| (b * 1_000_000.0) as u64),
        }
    }

    /// Record a cost.
    pub fn record(&self, cost_usd: f64) {
        let microdollars = (cost_usd * 1_000_000.0) as u64;
        self.total_cost_microdollars
            .fetch_add(microdollars, Ordering::Relaxed);
    }

    /// Get total cost in USD.
    pub fn total_cost_usd(&self) -> f64 {
        self.total_cost_microdollars.load(Ordering::Relaxed) as f64 / 1_000_000.0
    }

    /// Check if budget is exceeded.
    pub fn is_over_budget(&self) -> bool {
        if let Some(budget) = self.budget_microdollars {
            self.total_cost_microdollars.load(Ordering::Relaxed) >= budget
        } else {
            false
        }
    }

    /// Get remaining budget in USD (None if no budget set).
    pub fn remaining_usd(&self) -> Option<f64> {
        self.budget_microdollars.map(|budget| {
            let spent = self.total_cost_microdollars.load(Ordering::Relaxed);
            if spent >= budget {
                0.0
            } else {
                (budget - spent) as f64 / 1_000_000.0
            }
        })
    }

    /// Reset the tracker.
    pub fn reset(&self) {
        self.total_cost_microdollars.store(0, Ordering::Relaxed);
    }
}

impl Default for CostTracker {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrency_limiter() {
        let limiter = ConcurrencyLimiter::new(2);

        let result = limiter.execute(async { 42 }).await;
        assert_eq!(result, 42);
        assert_eq!(limiter.active_count(), 0);
    }

    #[test]
    fn test_cost_tracker() {
        let tracker = CostTracker::new(Some(1.0)); // $1 budget

        tracker.record(0.50);
        assert!((tracker.total_cost_usd() - 0.50).abs() < 0.001);
        assert!(!tracker.is_over_budget());

        tracker.record(0.60);
        assert!(tracker.is_over_budget());
    }

    #[test]
    fn test_cost_tracker_no_budget() {
        let tracker = CostTracker::new(None);
        tracker.record(100.0);
        assert!(!tracker.is_over_budget());
        assert!(tracker.remaining_usd().is_none());
    }
}
