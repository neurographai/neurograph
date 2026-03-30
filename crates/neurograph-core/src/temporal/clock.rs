// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Hybrid logical clock for strict temporal ordering.
//!
//! Combines wall time with a process-global monotonic counter to ensure
//! that two events with identical wall times still have a deterministic order.
//! This is critical for the temporal engine — without it, concurrent ingestion
//! can produce ambiguous fact orderings.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// Global atomic counter for the logical clock (process-wide).
static GLOBAL_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A hybrid logical clock tick.
///
/// Wall time provides real-world ordering; the monotonic counter breaks ties
/// when two events occur within the same chrono resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicalClock {
    /// Monotonic counter (process-global, never decreases).
    counter: u64,
    /// Wall time component.
    wall_time: DateTime<Utc>,
}

impl LogicalClock {
    /// Create a new clock tick at the current moment.
    pub fn now() -> Self {
        Self {
            counter: GLOBAL_COUNTER.fetch_add(1, Ordering::SeqCst),
            wall_time: Utc::now(),
        }
    }

    /// Create a clock tick at a specific wall time.
    pub fn at(time: DateTime<Utc>) -> Self {
        Self {
            counter: GLOBAL_COUNTER.fetch_add(1, Ordering::SeqCst),
            wall_time: time,
        }
    }

    /// Returns the wall time component.
    pub fn wall_time(&self) -> DateTime<Utc> {
        self.wall_time
    }

    /// Returns the monotonic counter component.
    pub fn counter(&self) -> u64 {
        self.counter
    }

    /// Compare two clock values. Wall time takes priority;
    /// counter breaks ties.
    pub fn happened_before(&self, other: &LogicalClock) -> bool {
        if self.wall_time != other.wall_time {
            self.wall_time < other.wall_time
        } else {
            self.counter < other.counter
        }
    }
}

impl PartialEq for LogicalClock {
    fn eq(&self, other: &Self) -> bool {
        self.counter == other.counter && self.wall_time == other.wall_time
    }
}

impl Eq for LogicalClock {}

impl PartialOrd for LogicalClock {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LogicalClock {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.wall_time
            .cmp(&other.wall_time)
            .then(self.counter.cmp(&other.counter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_ordering() {
        let c1 = LogicalClock::now();
        let c2 = LogicalClock::now();
        assert!(c1.happened_before(&c2));
        assert!(c1 < c2);
    }

    #[test]
    fn test_clock_monotonic_counter() {
        let c1 = LogicalClock::now();
        let c2 = LogicalClock::now();
        assert!(c2.counter() > c1.counter());
    }

    #[test]
    fn test_clock_at_specific_time() {
        let past = Utc::now() - chrono::Duration::days(30);
        let c = LogicalClock::at(past);
        assert_eq!(c.wall_time(), past);
    }

    #[test]
    fn test_clock_equality() {
        let c1 = LogicalClock::now();
        let c2 = c1.clone();
        assert_eq!(c1, c2);
    }
}
