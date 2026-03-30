// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Query budget tracking and enforcement.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::llm::traits::LlmUsage;

/// Tracks cost across all operations and enforces budget limits.
#[derive(Debug)]
pub struct QueryBudget {
    /// Maximum cost per query in USD (None = unlimited).
    max_cost_per_query: Option<f64>,
    /// Running total cost for the current query.
    current_cost_cents: AtomicU64, // Store in hundredths of cents for atomicity
}

impl QueryBudget {
    /// Create a new budget with optional per-query limit.
    pub fn new(max_cost_per_query: Option<f64>) -> Self {
        Self {
            max_cost_per_query,
            current_cost_cents: AtomicU64::new(0),
        }
    }

    /// Record a cost and check if budget is exceeded.
    pub fn record_cost(&self, cost_usd: f64) -> Result<(), BudgetError> {
        let cents = (cost_usd * 1_000_000.0) as u64;
        let new_total = self.current_cost_cents.fetch_add(cents, Ordering::SeqCst) + cents;
        let total_usd = new_total as f64 / 1_000_000.0;

        if let Some(max) = self.max_cost_per_query {
            if total_usd > max {
                return Err(BudgetError::Exceeded {
                    spent: total_usd,
                    limit: max,
                });
            }
        }

        Ok(())
    }

    /// Record cost from an LLM usage record.
    pub fn record_usage(&self, usage: &LlmUsage) -> Result<(), BudgetError> {
        self.record_cost(usage.cost_usd)
    }

    /// Get the current total cost.
    pub fn current_cost_usd(&self) -> f64 {
        self.current_cost_cents.load(Ordering::SeqCst) as f64 / 1_000_000.0
    }

    /// Check if we can afford an estimated cost.
    pub fn can_afford(&self, estimated_cost: f64) -> bool {
        if let Some(max) = self.max_cost_per_query {
            self.current_cost_usd() + estimated_cost <= max
        } else {
            true
        }
    }

    /// Reset the budget counter (for a new query).
    pub fn reset(&self) {
        self.current_cost_cents.store(0, Ordering::SeqCst);
    }
}

/// Budget errors.
#[derive(Debug, thiserror::Error)]
pub enum BudgetError {
    #[error("Budget exceeded: ${spent:.4} of ${limit:.4} used")]
    Exceeded { spent: f64, limit: f64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_tracking() {
        let budget = QueryBudget::new(Some(0.10)); // $0.10 limit

        assert!(budget.record_cost(0.03).is_ok());
        assert!(budget.record_cost(0.04).is_ok());
        assert!((budget.current_cost_usd() - 0.07).abs() < 0.001);

        assert!(budget.record_cost(0.05).is_err()); // Exceeds $0.10
    }

    #[test]
    fn test_unlimited_budget() {
        let budget = QueryBudget::new(None);
        assert!(budget.record_cost(100.0).is_ok());
        assert!(budget.can_afford(1000.0));
    }

    #[test]
    fn test_budget_reset() {
        let budget = QueryBudget::new(Some(0.10));
        budget.record_cost(0.08).unwrap();
        budget.reset();
        assert!(budget.current_cost_usd() < 0.001);
    }
}
