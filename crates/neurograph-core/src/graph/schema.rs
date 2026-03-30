// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Graph schema registry — tracks all types present in the graph.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::ontology::Ontology;

/// Registry of all entity types, relationship types, and their counts
/// currently present in the graph.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraphSchema {
    /// Ontology defining prescribed and learned types.
    pub ontology: Ontology,

    /// Count of entities per type.
    pub entity_type_counts: HashMap<String, usize>,

    /// Count of relationships per type.
    pub relationship_type_counts: HashMap<String, usize>,

    /// Set of all unique entity types seen.
    pub known_entity_types: HashSet<String>,

    /// Set of all unique relationship types seen.
    pub known_relationship_types: HashSet<String>,

    /// Total number of entities.
    pub total_entities: usize,

    /// Total number of relationships.
    pub total_relationships: usize,

    /// Total number of episodes.
    pub total_episodes: usize,

    /// Total number of communities.
    pub total_communities: usize,
}

impl GraphSchema {
    /// Create a new schema with the given ontology.
    pub fn new(ontology: Ontology) -> Self {
        Self {
            ontology,
            ..Default::default()
        }
    }

    /// Record an entity type occurrence.
    pub fn record_entity_type(&mut self, entity_type: &str) {
        *self
            .entity_type_counts
            .entry(entity_type.to_string())
            .or_insert(0) += 1;
        self.known_entity_types.insert(entity_type.to_string());
        self.total_entities += 1;

        // Also record as learned type in ontology
        self.ontology
            .record_learned_entity_type(entity_type);
    }

    /// Record a relationship type occurrence.
    pub fn record_relationship_type(&mut self, rel_type: &str) {
        *self
            .relationship_type_counts
            .entry(rel_type.to_string())
            .or_insert(0) += 1;
        self.known_relationship_types.insert(rel_type.to_string());
        self.total_relationships += 1;

        self.ontology
            .record_learned_relationship_type(rel_type);
    }

    /// Get a summary string of the schema.
    pub fn summary(&self) -> String {
        format!(
            "Schema: {} entities ({} types), {} relationships ({} types), {} episodes, {} communities",
            self.total_entities,
            self.known_entity_types.len(),
            self.total_relationships,
            self.known_relationship_types.len(),
            self.total_episodes,
            self.total_communities,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_tracking() {
        let mut schema = GraphSchema::default();

        schema.record_entity_type("Person");
        schema.record_entity_type("Person");
        schema.record_entity_type("Organization");

        assert_eq!(schema.entity_type_counts["Person"], 2);
        assert_eq!(schema.entity_type_counts["Organization"], 1);
        assert_eq!(schema.total_entities, 3);
        assert_eq!(schema.known_entity_types.len(), 2);
    }
}
