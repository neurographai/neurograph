// Copyright 2026 NeuroGraph Contributors
// SPDX-License-Identifier: Apache-2.0

//! Ontology system for prescribed and learned entity/relationship types.
//!
//! Influenced by Cognee's typed ontology system and Graphiti's
//! `entity_types` parameter in `add_episode()`.
//!
//! Users can define explicit ontologies (e.g., biomedical: Gene, Protein, Disease)
//! or let the system auto-discover types from data.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Definition of an entity type in the ontology.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTypeDefinition {
    /// Type name (e.g., "Person", "Organization").
    pub name: String,

    /// Description for LLM guidance during extraction.
    pub description: String,

    /// Expected attributes for this entity type.
    pub expected_attributes: Vec<AttributeDefinition>,

    /// Example entities of this type (for few-shot extraction).
    pub examples: Vec<String>,
}

impl EntityTypeDefinition {
    /// Create a new entity type definition.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            expected_attributes: Vec::new(),
            examples: Vec::new(),
        }
    }

    /// Add an expected attribute.
    pub fn with_attribute(mut self, attr: AttributeDefinition) -> Self {
        self.expected_attributes.push(attr);
        self
    }

    /// Add an example entity.
    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }
}

/// Definition of a relationship type in the ontology.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipTypeDefinition {
    /// Relationship type name (e.g., "WORKS_AT", "TREATS").
    pub name: String,

    /// Description for LLM guidance.
    pub description: String,

    /// Allowed source entity types.
    pub source_types: Vec<String>,

    /// Allowed target entity types.
    pub target_types: Vec<String>,

    /// Expected attributes on this relationship type.
    pub expected_attributes: Vec<AttributeDefinition>,
}

impl RelationshipTypeDefinition {
    /// Create a new relationship type definition.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        source_types: Vec<String>,
        target_types: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            source_types,
            target_types,
            expected_attributes: Vec::new(),
        }
    }
}

/// Definition of an attribute on entity/relationship types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeDefinition {
    /// Attribute name.
    pub name: String,

    /// Human-readable description.
    pub description: String,

    /// Data type hint (e.g., "string", "number", "date", "boolean").
    pub data_type: String,

    /// Whether this attribute is required.
    pub required: bool,
}

impl AttributeDefinition {
    pub fn new(name: impl Into<String>, data_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            data_type: data_type.into(),
            required: false,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }
}

/// The complete ontology for a knowledge graph.
///
/// Supports two modes:
/// - **Prescribed**: User defines entity types, relationship types, and constraints
/// - **Learned**: System auto-discovers types from data and presents for approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ontology {
    /// Prescribed entity type definitions.
    pub entity_types: HashMap<String, EntityTypeDefinition>,

    /// Prescribed relationship type definitions.
    pub relationship_types: HashMap<String, RelationshipTypeDefinition>,

    /// Auto-discovered entity types (from LLM extraction).
    pub learned_entity_types: Vec<String>,

    /// Auto-discovered relationship types.
    pub learned_relationship_types: Vec<String>,

    /// Whether to enforce the prescribed ontology strictly.
    /// If true, only prescribed types are allowed.
    /// If false, the LLM can create new types beyond the prescribed ones.
    pub strict_mode: bool,
}

impl Ontology {
    /// Create an empty ontology (no constraints — fully auto-discovered).
    pub fn open() -> Self {
        Self {
            entity_types: HashMap::new(),
            relationship_types: HashMap::new(),
            learned_entity_types: Vec::new(),
            learned_relationship_types: Vec::new(),
            strict_mode: false,
        }
    }

    /// Create a strict ontology (only prescribed types allowed).
    pub fn strict() -> Self {
        Self {
            entity_types: HashMap::new(),
            relationship_types: HashMap::new(),
            learned_entity_types: Vec::new(),
            learned_relationship_types: Vec::new(),
            strict_mode: true,
        }
    }

    /// Add an entity type definition.
    pub fn add_entity_type(&mut self, def: EntityTypeDefinition) {
        self.entity_types.insert(def.name.clone(), def);
    }

    /// Add a relationship type definition.
    pub fn add_relationship_type(&mut self, def: RelationshipTypeDefinition) {
        self.relationship_types.insert(def.name.clone(), def);
    }

    /// Record a learned entity type from extraction.
    pub fn record_learned_entity_type(&mut self, entity_type: impl Into<String>) {
        let t = entity_type.into();
        if !self.learned_entity_types.contains(&t) {
            self.learned_entity_types.push(t);
        }
    }

    /// Record a learned relationship type from extraction.
    pub fn record_learned_relationship_type(&mut self, rel_type: impl Into<String>) {
        let t = rel_type.into();
        if !self.learned_relationship_types.contains(&t) {
            self.learned_relationship_types.push(t);
        }
    }

    /// Generate extraction prompts for the LLM based on the ontology.
    pub fn to_extraction_prompt(&self) -> String {
        let mut prompt = String::new();

        if !self.entity_types.is_empty() {
            prompt.push_str("## Entity Types\n");
            for (name, def) in &self.entity_types {
                prompt.push_str(&format!("- **{}**: {}\n", name, def.description));
                for attr in &def.expected_attributes {
                    prompt.push_str(&format!(
                        "  - {}: {} ({}{})\n",
                        attr.name,
                        attr.description,
                        attr.data_type,
                        if attr.required { ", required" } else { "" }
                    ));
                }
            }
        }

        if !self.relationship_types.is_empty() {
            prompt.push_str("\n## Relationship Types\n");
            for (name, def) in &self.relationship_types {
                prompt.push_str(&format!(
                    "- **{}**: {} (from {:?} to {:?})\n",
                    name, def.description, def.source_types, def.target_types
                ));
            }
        }

        if self.strict_mode {
            prompt.push_str(
                "\nIMPORTANT: Only use the entity and relationship types listed above.\n",
            );
        }

        prompt
    }
}

impl Default for Ontology {
    fn default() -> Self {
        Self::open()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ontology_creation() {
        let mut ont = Ontology::strict();

        ont.add_entity_type(
            EntityTypeDefinition::new("Person", "A human being")
                .with_example("Alice")
                .with_attribute(
                    AttributeDefinition::new("age", "number").with_description("Age in years"),
                ),
        );

        ont.add_relationship_type(RelationshipTypeDefinition::new(
            "WORKS_AT",
            "Employment relationship",
            vec!["Person".into()],
            vec!["Organization".into()],
        ));

        assert!(ont.entity_types.contains_key("Person"));
        assert!(ont.relationship_types.contains_key("WORKS_AT"));

        let prompt = ont.to_extraction_prompt();
        assert!(prompt.contains("Person"));
        assert!(prompt.contains("WORKS_AT"));
        assert!(prompt.contains("Only use the entity"));
    }
}
