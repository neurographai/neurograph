// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Fact validation and schema enforcement.
//!
//! Provides optional validation layers:
//! - Schema validation against ontology definitions
//! - Basic fact plausibility checks

use crate::graph::ontology::Ontology;
use crate::ingestion::extractors::traits::{ExtractedEntity, ExtractedRelationship};

/// Validation result for an entity or relationship.
#[derive(Debug)]
pub struct ValidationResult {
    /// Whether the item passed validation.
    pub is_valid: bool,
    /// Warnings (non-blocking issues).
    pub warnings: Vec<String>,
    /// Errors (blocking issues).
    pub errors: Vec<String>,
}

impl ValidationResult {
    /// Create a passing result.
    pub fn pass() -> Self {
        Self {
            is_valid: true,
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Create a failing result.
    pub fn fail(error: impl Into<String>) -> Self {
        Self {
            is_valid: false,
            warnings: Vec::new(),
            errors: vec![error.into()],
        }
    }

    /// Add a warning.
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }
}

/// Validates extracted entities and relationships against an ontology.
pub struct SchemaValidator {
    ontology: Ontology,
}

impl SchemaValidator {
    /// Create a new validator with an ontology.
    pub fn new(ontology: Ontology) -> Self {
        Self { ontology }
    }

    /// Validate an extracted entity against the ontology.
    ///
    /// Checks:
    /// - Entity name is not empty
    /// - Entity type is valid according to ontology (if prescribed)
    pub fn validate_entity(&self, entity: &ExtractedEntity) -> ValidationResult {
        if entity.name.trim().is_empty() {
            return ValidationResult::fail("Entity name is empty");
        }

        if entity.name.len() > 500 {
            return ValidationResult::fail(format!(
                "Entity name too long: {} chars (max: 500)",
                entity.name.len()
            ));
        }

        // Check entity type against ontology (if prescribed)
        let valid_types = &self.ontology.entity_types;
        if !valid_types.is_empty() && !valid_types.contains_key(&entity.entity_type) {
            return ValidationResult::pass().with_warning(format!(
                "Entity type '{}' not in ontology, will be auto-registered",
                entity.entity_type
            ));
        }

        ValidationResult::pass()
    }

    /// Validate an extracted relationship.
    ///
    /// Checks:
    /// - Source and target entity names are not empty
    /// - Relationship type is not empty
    /// - Fact is not empty
    /// - Confidence is in valid range
    pub fn validate_relationship(&self, rel: &ExtractedRelationship) -> ValidationResult {
        if rel.source_entity.trim().is_empty() {
            return ValidationResult::fail("Source entity name is empty");
        }

        if rel.target_entity.trim().is_empty() {
            return ValidationResult::fail("Target entity name is empty");
        }

        if rel.relationship_type.trim().is_empty() {
            return ValidationResult::fail("Relationship type is empty");
        }

        if rel.fact.trim().is_empty() {
            return ValidationResult::fail("Fact is empty");
        }

        if !(0.0..=1.0).contains(&rel.confidence) {
            return ValidationResult::pass().with_warning(format!(
                "Confidence {} out of range [0.0, 1.0], clamping",
                rel.confidence
            ));
        }

        // Check relationship type against ontology
        let valid_rel_types = &self.ontology.relationship_types;
        if !valid_rel_types.is_empty() && !valid_rel_types.contains_key(&rel.relationship_type) {
            return ValidationResult::pass().with_warning(format!(
                "Relationship type '{}' not in ontology, will be auto-registered",
                rel.relationship_type
            ));
        }

        ValidationResult::pass()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_entity() {
        let validator = SchemaValidator::new(Ontology::open());
        let entity = ExtractedEntity {
            name: "Alice".to_string(),
            entity_type: "Person".to_string(),
            summary: "A researcher".to_string(),
        };
        let result = validator.validate_entity(&entity);
        assert!(result.is_valid);
    }

    #[test]
    fn test_empty_entity_name() {
        let validator = SchemaValidator::new(Ontology::open());
        let entity = ExtractedEntity {
            name: "".to_string(),
            entity_type: "Person".to_string(),
            summary: String::new(),
        };
        let result = validator.validate_entity(&entity);
        assert!(!result.is_valid);
    }

    #[test]
    fn test_valid_relationship() {
        let validator = SchemaValidator::new(Ontology::open());
        let rel = ExtractedRelationship {
            source_entity: "Alice".to_string(),
            target_entity: "Anthropic".to_string(),
            relationship_type: "WORKS_AT".to_string(),
            fact: "Alice works at Anthropic".to_string(),
            confidence: 0.95,
        };
        let result = validator.validate_relationship(&rel);
        assert!(result.is_valid);
    }
}
