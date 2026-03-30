// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Temporal graph view: nodes connected by temporal proximity and sequence.
//!
//! Uses Allen's Interval Algebra to classify temporal relations between
//! memory items (Before, After, During, Overlaps, Meets, etc.)

use super::*;
use std::collections::{BTreeMap, HashMap};
use chrono::Duration;

/// Temporal graph: edges represent temporal relationships between facts.
pub struct TemporalGraph {
    /// Temporal index: timestamp -> Vec<node_id>
    time_index: BTreeMap<DateTime<Utc>, Vec<Uuid>>,
    /// Sequence edges stored as (source, target, relation, gap_hours)
    sequence_edges: Vec<(Uuid, Uuid, TemporalRelation, i64)>,
    /// Validity windows for each node
    validity: HashMap<Uuid, (DateTime<Utc>, Option<DateTime<Utc>>)>,
    config: MultiGraphConfig,
}

/// Allen's Interval Algebra temporal relations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemporalRelation {
    Before,
    After,
    During,
    Overlaps,
    Meets,
    Starts,
    Finishes,
    Supersedes,
}

impl TemporalGraph {
    pub fn new(config: &MultiGraphConfig) -> Self {
        Self {
            time_index: BTreeMap::new(),
            sequence_edges: Vec::new(),
            validity: HashMap::new(),
            config: config.clone(),
        }
    }

    /// Add a memory item and create temporal edges to nearby items.
    pub fn add_item(&mut self, item: &MemoryItem) -> Vec<GraphEdge> {
        let mut new_edges = Vec::new();

        // Index by timestamp
        self.time_index
            .entry(item.valid_from)
            .or_default()
            .push(item.id);

        self.validity
            .insert(item.id, (item.valid_from, item.valid_until));

        // Find temporally adjacent nodes within window
        let window = Duration::days(self.config.temporal_window_days);
        let window_start = item.valid_from - window;
        let window_end = item
            .valid_until
            .unwrap_or(item.valid_from + window);

        let nearby: Vec<(DateTime<Utc>, Vec<Uuid>)> = self
            .time_index
            .range(window_start..=window_end)
            .map(|(dt, ids)| (*dt, ids.clone()))
            .collect();

        for (other_time, other_ids) in nearby {
            for other_id in other_ids {
                if other_id == item.id {
                    continue;
                }

                let other_end = self.validity.get(&other_id).and_then(|v| v.1);
                let relation = self.compute_relation(
                    item.valid_from,
                    item.valid_until,
                    other_time,
                    other_end,
                );

                let gap = if item.valid_from > other_time {
                    (item.valid_from - other_time).num_hours()
                } else {
                    (other_time - item.valid_from).num_hours()
                };

                // Weight inversely proportional to temporal distance
                let weight = 1.0 / (1.0 + gap as f64 / 24.0);

                self.sequence_edges
                    .push((item.id, other_id, relation.clone(), gap));

                new_edges.push(GraphEdge {
                    id: Uuid::new_v4(),
                    source: item.id,
                    target: other_id,
                    view: GraphView::Temporal,
                    relation: format!("{:?}", relation),
                    weight,
                    valid_from: item.valid_from,
                    valid_until: item.valid_until,
                    metadata: {
                        let mut m = HashMap::new();
                        m.insert(
                            "gap_hours".to_string(),
                            serde_json::json!(gap),
                        );
                        m
                    },
                });
            }
        }

        new_edges
    }

    /// Time-travel query: retrieve facts valid at a specific time point.
    pub fn traverse(&self, _query: &str, opts: &QueryOptions) -> SubgraphResult {
        let start = std::time::Instant::now();
        let time_point = opts.time_point.unwrap_or_else(Utc::now);

        // Find all nodes valid at the given time
        let mut valid_nodes: Vec<(Uuid, f64)> = Vec::new();

        for (id, (valid_from, valid_until)) in &self.validity {
            let is_valid = *valid_from <= time_point
                && valid_until.map(|end| time_point <= end).unwrap_or(true);

            if is_valid {
                let age = (time_point - *valid_from).num_days().max(0) as f64;
                let recency_score = 1.0 / (1.0 + age / 30.0);
                valid_nodes.push((*id, recency_score));
            }
        }

        valid_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        valid_nodes.truncate(opts.max_results);

        SubgraphResult {
            view: GraphView::Temporal,
            node_ids: valid_nodes.iter().map(|(id, _)| *id).collect(),
            scores: valid_nodes.iter().map(|(_, s)| *s).collect(),
            edges: Vec::new(),
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// Number of temporal edges.
    pub fn edge_count(&self) -> usize {
        self.sequence_edges.len()
    }

    /// Allen's Interval Algebra relation computation.
    fn compute_relation(
        &self,
        a_start: DateTime<Utc>,
        a_end: Option<DateTime<Utc>>,
        b_start: DateTime<Utc>,
        b_end: Option<DateTime<Utc>>,
    ) -> TemporalRelation {
        let a_end = a_end.unwrap_or(DateTime::<Utc>::MAX_UTC);
        let b_end = b_end.unwrap_or(DateTime::<Utc>::MAX_UTC);

        if a_end == b_start {
            TemporalRelation::Meets
        } else if a_start == b_start && a_end < b_end {
            TemporalRelation::Starts
        } else if a_end == b_end && a_start > b_start {
            TemporalRelation::Finishes
        } else if a_start >= b_start && a_end <= b_end {
            TemporalRelation::During
        } else if a_start < b_start && a_end > b_start && a_end < b_end {
            TemporalRelation::Overlaps
        } else if a_end <= b_start {
            TemporalRelation::Before
        } else {
            TemporalRelation::After
        }
    }
}
